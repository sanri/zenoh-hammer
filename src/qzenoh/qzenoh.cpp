#include "qzenoh.h"
#include <memory>
#include <QJsonArray>
#include <QJsonDocument>

QZenoh::QZenoh(ZConfig &&config, QObject *parent)
    : QObject(parent)
{
    z_owned_session_t session = z_open(&config.zConfig);
    zSession = session;
}

QZenoh::~QZenoh()
{
    z_close(&zSession);
}

bool QZenoh::checkOpen()
{
    return z_check(zSession);
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
