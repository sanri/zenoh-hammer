#pragma once

#include <QMainWindow>

#include "../qzenoh/qzenoh.h"

QT_BEGIN_NAMESPACE
namespace Ui
{
class MainWindow;
}
QT_END_NAMESPACE

class MainWindow: public QMainWindow
{
Q_OBJECT

public:
    MainWindow(QWidget *parent = nullptr);
    ~MainWindow();

public slots:
    void sessionOpen(ZConfig *config);
    void sessionClose();
    void newSubscriber(QString name, QString keyExpr);
    void delSubscriber(QString name);

signals:
    // 如果注册失败, 发送空指针
    void newSubscriberResult(QZSubscriber *subscriber);
    void delSubscriberResult(QString name);

private:
    void connect_signals_slots();
    void enableTabPage(bool b);

private:
    Ui::MainWindow *ui;
    QZenoh *qZenoh;
};
