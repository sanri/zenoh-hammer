#include "mainwindow.h"

#include <utility>
#include "./ui_mainwindow.h"

MainWindow::MainWindow(QWidget *parent)
    :
    QMainWindow(parent),
    ui(new Ui::MainWindow),
    qZenoh(nullptr)
{
    ui->setupUi(this);
    connect_signals_slots();
}

MainWindow::~MainWindow()
{
    delete ui;
}

void MainWindow::sessionOpen(ZConfig *config)
{
    auto zenoh = new QZenoh(config, this);
    if (zenoh->checkOpen()) {
        qZenoh = zenoh;
        enableTabPage(true);
    }
    else {
        ui->tab_session->setSessionPushButtonChecked(false);
        delete zenoh;
    }
}

void MainWindow::sessionClose()
{
    bool r = qZenoh->close();
    qDebug() << "qZenoh close() " << r;
    if (r) {
        delete qZenoh;
        qZenoh = nullptr;
        enableTabPage(false);
    }
    else {
        ui->tab_session->setSessionPushButtonChecked(true);
    }
}

void MainWindow::connect_signals_slots()
{
    connect(ui->tab_session, &PageSession::sessionOpen, this, &MainWindow::sessionOpen);
    connect(ui->tab_session, &PageSession::sessionClose, this, &MainWindow::sessionClose);
    connect(ui->tab_sub, &PageSub::newSubscriber, this, &MainWindow::newSubscriber);
    connect(this, &MainWindow::newSubscriberResult, ui->tab_sub, &PageSub::newSubscriberResult);
    connect(ui->tab_sub, &PageSub::delSubscriber, this, &MainWindow::delSubscriber);
    connect(this, &MainWindow::delSubscriberResult, ui->tab_sub, &PageSub::delSubscriberResult);
}

void MainWindow::enableTabPage(bool b)
{
    ui->tab_get->setEnabled(b);
    ui->tab_pub->setEnabled(b);
    ui->tab_put->setEnabled(b);
    ui->tab_sub->setEnabled(b);
}

void MainWindow::newSubscriber(QString name, QString keyExpr)
{
    if (qZenoh == nullptr) {
        emit newSubscriberResult(nullptr);
    }

    auto subscriber = new QZSubscriber(std::move(name), std::move(keyExpr));
    if (qZenoh->declareSubscriber(subscriber)) {
        emit newSubscriberResult(subscriber);
    }
    else {
        emit newSubscriberResult(nullptr);
    }
}

void MainWindow::delSubscriber(QString name)
{
    qZenoh->undeclareSubscriber(name);
    emit delSubscriberResult(name);
}
