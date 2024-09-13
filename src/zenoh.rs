use flume::{unbounded, RecvError, TryRecvError};
use std::{
    collections::BTreeMap,
    sync::Arc,
    thread,
    time::{Duration, SystemTime},
};
use tokio::{select, task, time::sleep};
use zenoh::{
    bytes::{Encoding, ZBytes},
    key_expr::OwnedKeyExpr,
    pubsub::Subscriber,
    qos::{CongestionControl, Priority, QoSBuilderTrait},
    query::{QueryConsolidation, QueryTarget, Reply},
    sample::{Locality, Sample},
    session::SessionDeclarations,
    Config, Session,
};

pub(crate) type Sender<T> = flume::Sender<T>;
pub(crate) type Receiver<T> = flume::Receiver<T>;

#[derive(Copy, Clone, Debug)]
pub enum KnownEncoding {
    ZBytes = 0,
    ZInt8 = 1,
    ZInt16 = 2,
    ZInt32 = 3,
    ZInt64 = 4,
    ZInt128 = 5,
    ZUint8 = 6,
    ZUint16 = 7,
    ZUint32 = 8,
    ZUint64 = 9,
    ZUint128 = 10,
    ZFloat32 = 11,
    ZFloat64 = 12,
    ZBool = 13,
    ZString = 14,
    ZError = 15,
    AppOctetStream = 16,
    TextPlain = 17,
    AppJson = 18,
    TextJson = 19,
    AppCdr = 20,
    AppCbor = 21,
    AppYaml = 22,
    TextYaml = 23,
    TextJson5 = 24,
    AppPythonSerializedObject = 25,
    AppProtobuf = 26,
    AppJavaSerializedObject = 27,
    AppOpenMetricsText = 28,
    ImagePng = 29,
    ImageJpeg = 30,
    ImageGif = 31,
    ImageBmp = 32,
    ImageWebP = 33,
    AppXml = 34,
    AppXWwwFormUrlencoded = 35,
    TextHtml = 36,
    TextXml = 37,
    TextCss = 38,
    TextJavascript = 39,
    TextMarkdown = 40,
    TextCsv = 41,
    AppSql = 42,
    AppCoapPayload = 43,
    AppJsonPathJson = 44,
    AppJsonSeq = 45,
    AppJsonPath = 46,
    AppJwt = 47,
    AppMp4 = 48,
    AppSoapXml = 49,
    AppYang = 50,
    AudioAac = 51,
    AudioFlac = 52,
    AudioMp4 = 53,
    AudioOgg = 54,
    AudioVorbis = 55,
    VideoH261 = 56,
    VideoH263 = 57,
    VideoH264 = 58,
    VideoH265 = 59,
    VideoH266 = 60,
    VideoMp4 = 61,
    VideoOgg = 62,
    VideoRaw = 63,
    VideoVp8 = 64,
    VideoVp9 = 65,
}

pub struct PutData {
    pub(crate) id: u64,
    pub(crate) key: OwnedKeyExpr,
    pub(crate) congestion_control: CongestionControl,
    pub(crate) priority: Priority,
    pub(crate) encoding: Encoding,
    pub(crate) payload: ZBytes,
}

pub struct QueryData {
    pub(crate) id: u64,
    pub(crate) key_expr: OwnedKeyExpr,
    pub(crate) target: QueryTarget,
    pub(crate) consolidation: QueryConsolidation,
    pub(crate) locality: Locality,
    pub(crate) timeout: Duration,
    pub(crate) encoding: Encoding,
    pub(crate) payload: Option<ZBytes>,
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
    SubCB(Box<(u64, Sample, SystemTime)>),     // (sub id, value, timestamp)
    GetRes(Box<(u64, Reply, SystemTime)>),     // (get id, result, timestamp)
    PutRes(Box<(u64, bool, String)>),          // true 表示成功， false表示失败
}

pub fn start_async(
    sender_to_gui: Sender<MsgZenohToGui>,
    receiver_from_gui: Receiver<MsgGuiToZenoh>,
    config: Config,
) {
    thread::spawn(move || {
        task::spawn_blocking(loop_zenoh(sender_to_gui, receiver_from_gui, config));
    });
}

async fn loop_zenoh(
    sender_to_gui: Sender<MsgZenohToGui>,
    receiver_from_gui: Receiver<MsgGuiToZenoh>,
    config: Config,
) {
    let session: Arc<Session> = match zenoh::open(config).await {
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
                let (id, key_expr) = *req;
                let subscriber: Subscriber<Receiver<Sample>> =
                    match session.declare_subscriber(key_expr).await {
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
                    .congestion_control(pd.congestion_control)
                    .priority(pd.priority)
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
        let r: Result<Sample, RecvError> = select!(
            sample = subscriber.recv_async() =>{
                 sample
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
    println!("task_subscriber exit");
}

async fn task_query(
    session: Arc<Session>,
    data: Box<QueryData>,
    sender_to_gui: Sender<MsgZenohToGui>,
) {
    println!("task_query entry");
    let d = *data;
    let replies = match d.payload {
        Some(v) => session.get(d.key_expr).payload(v),
        None => session.get(d.key_expr),
    }
    .target(d.target)
    .timeout(d.timeout)
    .consolidation(d.consolidation)
    .allowed_destination(d.locality)
    .await
    .unwrap();

    while let Ok(reply) = replies.recv_async().await {
        let t = SystemTime::now();
        let msg = MsgZenohToGui::GetRes(Box::new((d.id, reply, t)));
        if let Err(e) = sender_to_gui.send(msg) {
            println!("{}", e);
        }
    }

    println!("task_query exit");
}
