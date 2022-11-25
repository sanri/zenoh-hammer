#pragma once

#include <QMainWindow>
#include <QThread>

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
    void sessionOpen(QSharedPointer<ZConfig> config);
    void sessionClose();

private:
    void connect_signals_slots();
    void enableTabPage(bool b);

private:
    Ui::MainWindow *ui;
    QThread workerThread;
    QZenoh *qZenoh;
};
