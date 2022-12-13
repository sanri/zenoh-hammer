use async_std::task;
use flume::{unbounded, RecvError, TryRecvError};
use futures::select;
use std::{clone, collections::BTreeMap, sync::Arc, thread, time::Duration};

use zenoh::{
    prelude::{r#async::*, Config, KeyExpr, Sample, Session},
    subscriber::Subscriber,
};

pub(crate) type Sender<T> = flume::Sender<T>;
pub(crate) type Receiver<T> = flume::Receiver<T>;

pub struct PutData {
    pub(crate) id: u64,
    pub(crate) key: KeyExpr<'static>,
    pub(crate) congestion_control: CongestionControl,
    pub(crate) priority: Priority,
    pub(crate) value: Value,
}

pub enum MsgGuiToZenoh {
    Close,
    AddSubReq(Box<(u64, KeyExpr<'static>)>), // (sub id,key)
    DelSubReq(u64),                          // sub id
    GetReq(Box<KeyExpr<'static>>),
    AddPubReq,
    DelPubReq,
    PubReq,
    PutReq(Box<PutData>),
}

pub enum MsgZenohToGui {
    OpenSession(bool),         // true 表示成功， false 表示失败
    AddSubRes(u64),            // sub id
    DelSubRes(u64),            // sub id
    SubCB(Box<(u64, Sample)>), // (sub id, value)
    GetRes(Box<Sample>),
    AddPubRes,
    DelPubRes,
    PubRes,
    PutRes(Box<(u64, bool, String)>), // true 表示成功， false表示失败
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
                    session.declare_subscriber(key_expr).res().await.unwrap();
                let (close_sender, close_receiver): (Sender<()>, Receiver<()>) = unbounded();
                let _ = subscriber_senders.insert(id, close_sender);
                task::spawn(task_subscriber(
                    id,
                    subscriber,
                    close_receiver,
                    sender_to_gui.clone(),
                ));
                let _ = sender_to_gui.send(MsgZenohToGui::AddSubRes(id));
            }
            MsgGuiToZenoh::DelSubReq(id) => {
                if let Some(sender) = subscriber_senders.get(&id) {
                    let _ = sender.send(());
                }
                let _ = subscriber_senders.remove(&id);
                let _ = sender_to_gui.send(MsgZenohToGui::DelSubRes(id));
            }
            MsgGuiToZenoh::GetReq(_) => {}
            MsgGuiToZenoh::AddPubReq => {}
            MsgGuiToZenoh::DelPubReq => {}
            MsgGuiToZenoh::PubReq => {}
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
                if let Ok(sample) = sample{
                    let msg = MsgZenohToGui::SubCB(Box::new((id, sample)));
                    if let Err(e) = sender_to_gui.send(msg) {
                        println!("{}", e);
                    }
                }
            },

            r = close_receiver.recv_async() =>{
                    break 'a;
            },
        );
    }
    println!("task_subscriber exit");
}
