use async_std::task;
use flume::{unbounded, TryRecvError};
use futures::select;
use std::{collections::BTreeMap, sync::Arc, thread, time::Duration};

use zenoh::{
    prelude::{r#async::*, Config, Sample, Session},
    query::Reply,
    subscriber::Subscriber,
    time::new_reception_timestamp,
};

pub(crate) type Sender<T> = flume::Sender<T>;
pub(crate) type Receiver<T> = flume::Receiver<T>;

pub struct PutData {
    pub(crate) id: u64,
    pub(crate) key: OwnedKeyExpr,
    pub(crate) congestion_control: CongestionControl,
    pub(crate) priority: Priority,
    pub(crate) value: Value,
}

pub struct QueryData {
    pub(crate) id: u64,
    pub(crate) key_expr: OwnedKeyExpr,
    pub(crate) target: QueryTarget,
    pub(crate) consolidation: QueryConsolidation,
    pub(crate) locality: Locality,
    pub(crate) timeout: Duration,
    pub(crate) value: Option<Value>,
}

pub enum MsgGuiToZenoh {
    Close,
    AddSubReq(Box<(u64, OwnedKeyExpr)>), // (sub id,key)
    DelSubReq(u64),                      // sub id
    GetReq(Box<QueryData>),
    PutReq(Box<PutData>),
}

pub enum MsgZenohToGui {
    OpenSession(bool),                         // true 表示成功， false 表示失败
    AddSubRes(Box<(u64, Result<(), String>)>), // sub id, true 表示成功, false表示失败
    DelSubRes(u64),                            // sub id
    SubCB(Box<(u64, Sample)>),                 // (sub id, value)
    GetRes(Box<(u64, Reply)>),                 // (get id, result)
    PutRes(Box<(u64, bool, String)>),          // true 表示成功， false表示失败
}

pub fn start_async(
    sender_to_gui: Sender<MsgZenohToGui>,
    receiver_from_gui: Receiver<MsgGuiToZenoh>,
    config: Config,
) {
    thread::spawn(move || {
        task::block_on(loop_zenoh(sender_to_gui, receiver_from_gui, config));
    });
}

async fn loop_zenoh(
    sender_to_gui: Sender<MsgZenohToGui>,
    receiver_from_gui: Receiver<MsgGuiToZenoh>,
    config: Config,
) {
    let session: Arc<Session> = match zenoh::open(config).res().await {
        Ok(s) => Arc::new(s),
        Err(e) => {
            println!("{}", e);
            let _ = sender_to_gui.send(MsgZenohToGui::OpenSession(false));
            return;
        }
    };
    let _ = sender_to_gui.send(MsgZenohToGui::OpenSession(true));

    let mut subscriber_senders: BTreeMap<u64, Sender<()>> = BTreeMap::new();

    'a: loop {
        let try_read = receiver_from_gui.try_recv();
        let msg = match try_read {
            Ok(m) => m,
            Err(e) => {
                match e {
                    TryRecvError::Empty => {
                        task::sleep(Duration::from_millis(8)).await;
                        continue 'a;
                    }
                    TryRecvError::Disconnected => {
                        break 'a;
                    }
                };
            }
        };
        match msg {
            MsgGuiToZenoh::Close => {
                break 'a;
            }
            MsgGuiToZenoh::AddSubReq(req) => {
                let (id, key_expr) = *req;
                let subscriber: Subscriber<Receiver<Sample>> =
                    match session.declare_subscriber(key_expr).res().await {
                        Ok(o) => o,
                        Err(e) => {
                            let _ = sender_to_gui
                                .send(MsgZenohToGui::AddSubRes(Box::new((id, Err(e.to_string())))));
                            return;
                        }
                    };
                let (close_sender, close_receiver): (Sender<()>, Receiver<()>) = unbounded();
                let _ = subscriber_senders.insert(id, close_sender);
                task::spawn(task_subscriber(
                    id,
                    subscriber,
                    close_receiver,
                    sender_to_gui.clone(),
                ));
                let _ = sender_to_gui.send(MsgZenohToGui::AddSubRes(Box::new((id, Ok(())))));
            }
            MsgGuiToZenoh::DelSubReq(id) => {
                if let Some(sender) = subscriber_senders.get(&id) {
                    let _ = sender.send(());
                }
                let _ = subscriber_senders.remove(&id);
                let _ = sender_to_gui.send(MsgZenohToGui::DelSubRes(id));
            }
            MsgGuiToZenoh::GetReq(req) => {
                task::spawn(task_query(session.clone(), req, sender_to_gui.clone()));
            }
            MsgGuiToZenoh::PutReq(p) => {
                let pd = *p;
                if let Err(e) = session
                    .put(pd.key.clone(), pd.value)
                    .congestion_control(pd.congestion_control)
                    .priority(pd.priority)
                    .res()
                    .await
                {
                    let s = format!("{}", e);
                    let _ = sender_to_gui.send(MsgZenohToGui::PutRes(Box::new((pd.id, false, s))));
                } else {
                    let s = format!("{} put succeed", pd.key);
                    let _ = sender_to_gui.send(MsgZenohToGui::PutRes(Box::new((pd.id, true, s))));
                }
            }
        }
    } // loop 'a

    for (sub_id, sender) in subscriber_senders {
        let _ = sender.send(());
        let _ = sender_to_gui.send(MsgZenohToGui::DelSubRes(sub_id));
    }
}

async fn task_subscriber(
    id: u64,
    subscriber: Subscriber<'_, Receiver<Sample>>,
    close_receiver: Receiver<()>,
    sender_to_gui: Sender<MsgZenohToGui>,
) {
    println!("task_subscriber entry");
    'a: loop {
        select!(
            sample = subscriber.recv_async() =>{
                if let Ok(mut sample) = sample{
                    if sample.timestamp.is_none(){
                        sample.timestamp = Some(new_reception_timestamp());
                    }
                    let msg = MsgZenohToGui::SubCB(Box::new((id, sample)));
                    if let Err(e) = sender_to_gui.send(msg) {
                        println!("{}", e);
                    }
                }
            },

            _ = close_receiver.recv_async() =>{
                    break 'a;
            },
        );
    }
    println!("task_subscriber exit");
}

async fn task_query(
    session: Arc<Session>,
    data: Box<QueryData>,
    sender_to_gui: Sender<MsgZenohToGui>,
) {
    println!("task_query entry");
    let d = *data;
    let replies = match d.value {
        Some(v) => session.get(d.key_expr).with_value(v),
        None => session.get(d.key_expr),
    }
    .target(d.target)
    .timeout(d.timeout)
    .consolidation(d.consolidation)
    .allowed_destination(d.locality)
    .res()
    .await
    .unwrap();

    while let Ok(mut reply) = replies.recv_async().await {
        if let Ok(sample) = &mut reply.sample {
            if sample.timestamp.is_none() {
                sample.timestamp = Some(new_reception_timestamp());
            }
        }
        let msg = MsgZenohToGui::GetRes(Box::new((d.id, reply)));
        if let Err(e) = sender_to_gui.send(msg) {
            println!("{}", e);
        }
    }

    println!("task_query exit");
}
