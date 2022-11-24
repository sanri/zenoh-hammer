#pragma once

#include <QObject>
#include "zenoh.h"

enum ZMode
{
    peer,
    client,
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
    struct z_owned_config_t zConfig;
    friend class QZenoh;
};

class QZenoh: public QObject
{
Q_OBJECT
public:
    explicit QZenoh(ZConfig &&config , QObject *parent = nullptr);
    ~QZenoh() override;

    // 返回 true 说明open成功
    bool checkOpen();

private:
    struct z_owned_session_t zSession;
};


