#include "qzenoh.h"
#include <memory>
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
    z_close(zSession);
    delete zSession;
}

bool QZenoh::checkOpen()
{
    return z_check(*zSession);
}

bool QZenoh::declareSubscriber(QZSubscriber *subscriber)
{
    if (!checkOpen()) {
        return false;
    }

    z_owned_closure_sample_t callback;
    callback.context = (void *) subscriber;
    callback.call = QZSubscriber::callbackCall;
    callback.drop = nullptr;

    z_keyexpr_t key = z_keyexpr(subscriber->key.toStdString().c_str());

    auto sub = new z_owned_subscriber_t;
    *sub = z_declare_subscriber(z_session_loan(zSession), key, &callback, subscriber->opts);

    if (!z_check(*sub)) {
        delete sub;
        return false;
    }

    subscriber->subscriber = sub;

    return true;
}

void QZenoh::undeclareSubscriber(QString name)
{

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
    auto value = jd.toJson(QJsonDocument::JsonFormat::Compact).constData();
    int8_t r = zc_config_insert_json(cfg, key, value);
    return (r == 0);
}

bool ZConfig::setListens(const QList<QString> &endpoints)
{
    struct z_config_t cfg = z_config_loan(&zConfig);
    const char *key = "listen/endpoints";
    QJsonDocument jd = QJsonDocument(QJsonArray::fromStringList(endpoints));
    auto value = jd.toJson(QJsonDocument::JsonFormat::Compact).constData();
    int8_t r = zc_config_insert_json(cfg, key, value);
    return (r == 0);
}

void QZSubscriber::callbackCall(const z_sample_t *sample, void *context)
{
    auto subscriber = (QZSubscriber *) context;
    auto p = QSharedPointer<ZSample>(new ZSample(sample));
    emit subscriber->newSubMsg(p);
}

QZSubscriber::QZSubscriber(QString name, QString key, QObject *parent)
    : QObject(parent), name(std::move(name)), key(std::move(key))
{
    opts = new z_subscriber_options_t;
    opts->reliability = z_reliability_t::Z_RELIABILITY_RELIABLE;
}

QZSubscriber::~QZSubscriber()
{
    delete opts;
}
void QZSubscriber::setOptions(z_reliability_t reliability)
{
    opts->reliability = reliability;
}

ZSample::ZSample(const z_sample_t *sample)
    :
    timestamp(&sample->timestamp),
    encoding(sample->encoding.prefix)
{
    char *key = z_keyexpr_to_string(sample->keyexpr);
    keyexpr = QString(key);
    free(key);

    payload = QByteArray((char *) sample->payload.start, (qsizetype) sample->payload.len);
}

ZTimestamp::ZTimestamp(const z_timestamp_t *time)
{
    this->time = time->time;
    this->id = QByteArray((char *) time->id.start, (qsizetype) time->id.len);
}
