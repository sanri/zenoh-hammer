use flume::{unbounded, RecvError, TryRecvError};
use log::{error, info, warn};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    thread,
    time::{Duration, SystemTime},
};
use tokio::{runtime::Runtime, select, task, time::sleep};
use zenoh::query::{Parameters, Selector};
use zenoh::{
    bytes::{Encoding, ZBytes},
    handlers::FifoChannelHandler,
    key_expr::OwnedKeyExpr,
    pubsub::Subscriber,
    qos::{CongestionControl, Priority},
    query::{QueryConsolidation, QueryTarget, Reply},
    sample::{Locality, Sample},
    Config, Session,
};

pub type Sender<T> = flume::Sender<T>;
pub type Receiver<T> = flume::Receiver<T>;

pub struct SubData {
    pub id: u64,
    pub key_expr: OwnedKeyExpr,
    pub origin: Locality,
}

pub struct PutData {
    pub id: u64,
    pub key: OwnedKeyExpr,
    pub congestion_control: CongestionControl,
    pub priority: Priority,
    pub encoding: Encoding,
    pub payload: ZBytes,
}

pub struct QueryData {
    pub id: u64,
    pub key_expr: OwnedKeyExpr,
    pub parameters: Parameters<'static>,
    pub attachment: Option<ZBytes>,
    pub target: QueryTarget,
    pub consolidation: QueryConsolidation,
    pub locality: Locality,
    pub timeout: Duration,
    pub value: Option<(Encoding, ZBytes)>,
}

pub enum MsgGuiToZenoh {
    Close,
    AddSubReq(Box<SubData>), // (sub id,key)
    DelSubReq(u64),          // sub id
    GetReq(Box<QueryData>),
    PutReq(Box<PutData>),
}

pub enum MsgZenohToGui {
    OpenSession(Result<u64, (u64, String)>), // Ok表示成功， Err表示失败
    AddSubRes(Box<(u64, Result<(), String>)>), // sub id, true 表示成功, false表示失败
    DelSubRes(u64),                          // sub id
    SubCB(Box<(u64, Sample, SystemTime)>),   // (sub id, value, timestamp)
    GetRes(Box<(u64, Reply)>),               // (get id, result, timestamp)
    PutRes(Box<(u64, bool, String)>),        // true 表示成功， false表示失败
}

pub fn start_async(
    sender_to_gui: Sender<MsgZenohToGui>,
    receiver_from_gui: Receiver<MsgGuiToZenoh>,
    id: u64,
    config_file_path: PathBuf,
) {
    thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(loop_zenoh(
            sender_to_gui,
            receiver_from_gui,
            config_file_path,
            id,
        ));
    });
}

async fn loop_zenoh(
    sender_to_gui: Sender<MsgZenohToGui>,
    receiver_from_gui: Receiver<MsgGuiToZenoh>,
    config_file_path: PathBuf,
    id: u64,
) {
    let config = match Config::from_file(config_file_path) {
        Ok(o) => o,
        Err(e) => {
            let s = e.to_string();
            warn!("{s}");
            let _ = sender_to_gui.send(MsgZenohToGui::OpenSession(Err((id, s))));
            return;
        }
    };

    let session: Session = match zenoh::open(config).await {
        Ok(o) => {
            info!("session open ok");
            o
        }
        Err(e) => {
            let s = e.to_string();
            warn!("{s}");
            let _ = sender_to_gui.send(MsgZenohToGui::OpenSession(Err((id, s))));
            return;
        }
    };
    let _ = sender_to_gui.send(MsgZenohToGui::OpenSession(Ok(id)));

    let mut subscriber_senders: BTreeMap<u64, Sender<()>> = BTreeMap::new();

    'a: loop {
        let try_read = receiver_from_gui.try_recv();
        let msg = match try_read {
            Ok(m) => m,
            Err(e) => {
                match e {
                    TryRecvError::Empty => {
                        sleep(Duration::from_millis(8)).await;
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
                let SubData {
                    id,
                    key_expr,
                    origin,
                } = *req;
                let subscriber: Subscriber<FifoChannelHandler<Sample>> = match session
                    .declare_subscriber(key_expr)
                    .allowed_origin(origin)
                    .await
                {
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
                    .put(pd.key.clone(), pd.payload)
                    .encoding(pd.encoding)
                    .congestion_control(pd.congestion_control)
                    .priority(pd.priority)
                    .await
                {
                    let s = format!("put error \"{}\", {}", pd.key, e);
                    warn!("{s}");
                    let _ = sender_to_gui.send(MsgZenohToGui::PutRes(Box::new((pd.id, false, s))));
                } else {
                    let s = format!("put ok \"{}\"", pd.key);
                    info!("{s}");
                    let _ = sender_to_gui.send(MsgZenohToGui::PutRes(Box::new((pd.id, true, s))));
                }
            }
        }
    }

    for (sub_id, sender) in subscriber_senders {
        let _ = sender.send(());
        let _ = sender_to_gui.send(MsgZenohToGui::DelSubRes(sub_id));
    }

    info!("session closed");
}

async fn task_subscriber(
    id: u64,
    subscriber: Subscriber<FifoChannelHandler<Sample>>,
    close_receiver: Receiver<()>,
    sender_to_gui: Sender<MsgZenohToGui>,
) {
    info!("task_subscriber entry");
    'a: loop {
        let r: Result<Sample, RecvError> = select!(
            sample = subscriber.recv_async() =>{
                 sample.map_err(|_|RecvError::Disconnected)
            },

            _ = close_receiver.recv_async() =>{
                 Err(RecvError::Disconnected)
            },
        );

        match r {
            Ok(sample) => {
                let t = SystemTime::now();
                let msg = MsgZenohToGui::SubCB(Box::new((id, sample, t)));
                if let Err(e) = sender_to_gui.send(msg) {
                    println!("{}", e);
                }
            }
            Err(_) => {
                break 'a;
            }
        }
    }
    info!("task_subscriber exit");
}

async fn task_query(session: Session, data: Box<QueryData>, sender_to_gui: Sender<MsgZenohToGui>) {
    let d = *data;
    let key_expr_str = d.key_expr.to_string();
    info!("task_query entry, key expr \"{}\"", key_expr_str);

    let selector = Selector::from((d.key_expr, d.parameters));
    let replies = match d.value {
        Some((encoding, payload)) => session.get(selector).payload(payload).encoding(encoding),
        None => session.get(selector),
    }
    .attachment(d.attachment)
    .target(d.target)
    .timeout(d.timeout)
    .consolidation(d.consolidation)
    .allowed_destination(d.locality)
    .await
    .unwrap();

    while let Ok(reply) = replies.recv_async().await {
        let msg = MsgZenohToGui::GetRes(Box::new((d.id, reply)));
        if let Err(e) = sender_to_gui.send(msg) {
            error!("{}", e);
        }
    }

    info!("task_query exit, key expr {}", key_expr_str);
}
