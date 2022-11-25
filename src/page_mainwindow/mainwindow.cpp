#include "mainwindow.h"
#include "./ui_mainwindow.h"

MainWindow::MainWindow(QWidget *parent)
    :
    QMainWindow(parent),
    ui(new Ui::MainWindow),
    qZenoh(nullptr)
{
    ui->setupUi(this);
    enableTabPage(false);
    connect_signals_slots();
}

MainWindow::~MainWindow()
{
    delete ui;
}

void MainWindow::sessionOpen(QSharedPointer<ZConfig> config)
{
    auto zenoh = new QZenoh(config.data(), this);
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

    enableTabPage(false);
}

void MainWindow::connect_signals_slots()
{
    connect(ui->tab_session, &PageSession::sessionOpen, this, &MainWindow::sessionOpen);
    connect(ui->tab_session, &PageSession::sessionClose, this, &MainWindow::sessionClose);
}

void MainWindow::enableTabPage(bool b)
{
    ui->tab_get->setEnabled(b);
    ui->tab_pub->setEnabled(b);
    ui->tab_put->setEnabled(b);
    ui->tab_sub->setEnabled(b);
}
