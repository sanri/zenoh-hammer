#pragma once

#include <QWidget>
#include <QSharedPointer>
#include "../qzenoh/qzenoh.h"

QT_BEGIN_NAMESPACE
namespace Ui
{
class PageSession;
}
QT_END_NAMESPACE

class PageSession: public QWidget
{
Q_OBJECT

public:
    explicit PageSession(QWidget *parent = nullptr);
    ~PageSession() override;

public slots:
    void showConfig(ZConfig &zConfig);

public:
    ZConfig *getZConfig();
    void setSessionPushButtonChecked(bool b);

signals:
    void sessionOpen(QSharedPointer<ZConfig> config);
    void sessionClose();

private slots:
    void update_clicked(bool checked);
    void connectAdd_clicked(bool checked);
    void connectDel_clicked(bool checked);
    void listenAdd_clicked(bool checked);
    void listenDel_clicked(bool checked);
    void sessionPushButton_clicked(bool checked);

private:
    void connect_signals_slots();
    bool checkAndSetConfig(ZConfig &zConfig);
    bool setConnects(ZConfig &zConfig);
    bool setListens(ZConfig &zConfig);
    bool setMode(ZConfig &zConfig);

private:
    Ui::PageSession *ui;
};

