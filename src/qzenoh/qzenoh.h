#pragma once

#include <QObject>
#include <QMap>
#include <QSharedPointer>
#include "zenoh.h"

enum ZMode
{
    peer,
    client,
};

class ZTimestamp
{
public:
    explicit ZTimestamp(const z_timestamp_t *time);
    ~ZTimestamp() = default;

private:
    uint64_t time;
    QByteArray id;
};

class ZConfig
{
public:
    ZConfig();
    ~ZConfig();

    QString getStr();
    bool setMode(ZMode mode);
    bool setConnects(const QList<QString> &endpoints);
    bool setListens(const QList<QString> &endpoints);

private:
    z_owned_config_t zConfig;
    friend class QZenoh;
};

class ZSample
{
public:
    explicit ZSample(const z_sample_t *sample);
    ~ZSample() = default;

private:
    QString keyexpr;
    QByteArray payload;
    ZTimestamp timestamp;
    z_encoding_prefix_t encoding;
};

class QZSubscriber: public QObject
{
Q_OBJECT
public:
    explicit QZSubscriber(QString name, QString key, QObject *parent = nullptr);
    ~QZSubscriber() override;

    void setOptions(z_reliability_t opt);

signals:
    void newSubMsg(const QSharedPointer<ZSample> sample);

private:
    static void callbackCall(const z_sample_t *sample, void *context);

private:
    QString name;
    QString key;
    z_subscriber_options_t *opts;
    z_owned_subscriber_t *subscriber = nullptr;
    friend class QZenoh;
};

class QZenoh: public QObject
{
Q_OBJECT
public:
    explicit QZenoh(ZConfig *config, QObject *parent = nullptr);
    ~QZenoh() override;

    // 返回 true 说明open成功
    bool checkOpen();

    // 增加一个订阅
    // sub 的内存管理转移给 QZenoh, 不要在调用此函数后释放 subscriber
    bool declareSubscriber(QZSubscriber *subscriber);

    // 取消一个订阅
    void undeclareSubscriber(QString name);

private:
    z_owned_session_t *zSession = nullptr;
    QMap<QString, QZSubscriber *> mapSub;
};
