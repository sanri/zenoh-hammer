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
    ZTimestamp() = default;
    ZTimestamp(uint32_t secs, uint32_t ms);
    ~ZTimestamp() = default;

    uint32_t getSecs() const;
    void setSecs(uint32_t secs);
    uint32_t getMsec() const;
    void setMsec(uint32_t msec);
    QString format() const;

private:
    uint64_t time = 0;
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

    QString getKey() const;

private:
    QString key;
    ZTimestamp timestamp;
    QByteArray payload;
    z_encoding_prefix_t encoding;
    friend class SubDataItem;
};

class QZSubscriber: public QObject
{
Q_OBJECT
public:
    explicit QZSubscriber(QString name, QString key, QObject *parent = nullptr);
    ~QZSubscriber() override;

    void setOptions(z_reliability_t opt);
    QString getName();
    QString getKeyExpr();

signals:
    void newSubMsg(QString name, const QSharedPointer<ZSample> sample);

private:
    static void callbackCall(const z_sample_t *sample, void *context);

private:
    QString name;
    QString keyExpr;
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

    bool close();

    // 增加一个订阅
    // sub 的内存管理转移给 QZenoh, 不要在调用此函数后释放 subscriber
    bool declareSubscriber(QZSubscriber *subscriber);

    // 取消一个订阅
    void undeclareSubscriber(const QString &name);

private:
    z_owned_session_t *zSession = nullptr;
    // name
    QMap<QString, QZSubscriber *> mapSub;
};
