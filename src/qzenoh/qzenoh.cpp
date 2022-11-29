#include "qzenoh.h"
#include <memory>
#include <sstream>
#include <iomanip>
#include <QJsonArray>
#include <QJsonDocument>
#include <utility>

QZenoh::QZenoh(ZConfig *config, QObject *parent)
    : QObject(parent)
{
    auto session = new z_owned_session_t;
    *session = z_open(&config->zConfig);
    zSession = session;
}

QZenoh::~QZenoh()
{
    delete zSession;
}

bool QZenoh::checkOpen()
{
    return z_check(*zSession);
}

bool QZenoh::close()
{
    return (z_close(zSession) == 0);
}

bool QZenoh::declareSubscriber(QZSubscriber *subscriber)
{
    if (!checkOpen()) {
        return false;
    }

    if (mapSub.contains(subscriber->name)) {
        return false;
    }

    z_owned_closure_sample_t callback;
    callback.context = (void *) subscriber;
    callback.call = QZSubscriber::callbackCall;
    callback.drop = nullptr;

    z_keyexpr_t key = z_keyexpr(subscriber->keyExpr.toStdString().c_str());

    auto sub = new z_owned_subscriber_t;
    *sub = z_declare_subscriber(z_session_loan(zSession), key, &callback, subscriber->opts);

    if (!z_check(*sub)) {
        delete sub;
        return false;
    }

    subscriber->subscriber = sub;

    mapSub.insert(subscriber->name, subscriber);

    return true;
}

void QZenoh::undeclareSubscriber(const QString &name)
{
    auto it = mapSub.find(name);
    if (it == mapSub.end()) {
        return;
    }
    disconnect(it.value(), nullptr, nullptr, nullptr);

    z_undeclare_subscriber(it.value()->subscriber);
    delete it.value();

    mapSub.remove(name);
}

ZConfig::ZConfig()
{
    zConfig = z_config_default();
}

ZConfig::~ZConfig()
{
    z_config_drop(&zConfig);
}

QString ZConfig::getStr()
{
    struct z_config_t cfg = z_config_loan(&zConfig);
    char *p = zc_config_to_string(cfg);
    QString out = QString(p);
    free(p);
    return out;
}

bool ZConfig::setMode(ZMode mode)
{
    struct z_config_t cfg = z_config_loan(&zConfig);
    const char *key = "mode";
    const char *value = (mode == ZMode::client) ? "\"client\"" : "\"peer\"";
    int8_t r = zc_config_insert_json(cfg, key, value);
    return (r == 0);
}

bool ZConfig::setConnects(const QList<QString> &endpoints)
{
    struct z_config_t cfg = z_config_loan(&zConfig);
    const char *key = "connect/endpoints";
    QJsonDocument jd = QJsonDocument(QJsonArray::fromStringList(endpoints));
    auto v = jd.toJson(QJsonDocument::JsonFormat::Compact);
    auto value = v.constData();
    int8_t r = zc_config_insert_json(cfg, key, value);
    return (r == 0);
}

bool ZConfig::setListens(const QList<QString> &endpoints)
{
    struct z_config_t cfg = z_config_loan(&zConfig);
    const char *key = "listen/endpoints";
    QJsonDocument jd = QJsonDocument(QJsonArray::fromStringList(endpoints));
    auto v = jd.toJson(QJsonDocument::JsonFormat::Compact);
    auto value = v.constData();
    int8_t r = zc_config_insert_json(cfg, key, value);
    return (r == 0);
}

void QZSubscriber::callbackCall(const z_sample_t *sample, void *context)
{
    auto subscriber = (QZSubscriber *) context;
    auto p = QSharedPointer<ZSample>(new ZSample(sample));
    emit subscriber->newSubMsg(subscriber->name, p);
}

QZSubscriber::QZSubscriber(QString name, QString key, QObject *parent)
    : QObject(parent), name(std::move(name)), keyExpr(std::move(key))
{
    opts = new z_subscriber_options_t;
    opts->reliability = z_reliability_t::Z_RELIABILITY_RELIABLE;
}

QZSubscriber::~QZSubscriber()
{
    delete opts;
    delete subscriber;
}
void QZSubscriber::setOptions(z_reliability_t reliability)
{
    opts->reliability = reliability;
}

QString QZSubscriber::getName()
{
    return name;
}

QString QZSubscriber::getKeyExpr()
{
    return keyExpr;
}

ZSample::ZSample(const z_sample_t *sample)
    :
    timestamp(&sample->timestamp),
    encoding(sample->encoding.prefix)
{
    char *k = z_keyexpr_to_string(sample->keyexpr);
    this->key = QString(k);
    free(k);

    payload = QByteArray((char *) sample->payload.start, (qsizetype) sample->payload.len);
}

QString ZSample::getKey() const
{
    return key;
}

ZTimestamp::ZTimestamp(const z_timestamp_t *time)
{
    this->time = time->time;
    this->id = QByteArray((char *) time->id.start, (qsizetype) time->id.len);
}

ZTimestamp::ZTimestamp(uint32_t secs, uint32_t ms)
{
    setSecs(secs);
    setMsec(ms);
}

uint32_t ZTimestamp::getSecs() const
{
    return (uint32_t) (time >> 32);
}

void ZTimestamp::setSecs(uint32_t secs)
{
    uint64_t t = ((uint64_t) secs) << 32;
    time = (time & 0x00000000ffffffff) | t;
}

uint32_t ZTimestamp::getMsec() const
{
    uint64_t frac = time & 0xFFFFFFFF;
    return (uint32_t) ((frac * 1000000000ull) / (1ull << 32));
}

void ZTimestamp::setMsec(uint32_t msec)
{
    uint64_t frac = ((uint64_t) msec) * (1ull << 32) / 1000000000ull;
    time = (time & 0xffffffff00000000) | frac;
}

QString ZTimestamp::format() const
{
    time_t t_c = getSecs();
    std::ostringstream o;
    o << std::put_time(std::localtime(&t_c), "%F %T.");
    int milltime = getMsec();
    QString datetime = QString(o.str().c_str()) + QString::number(milltime);
    return datetime;
}

