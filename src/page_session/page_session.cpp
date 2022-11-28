#include "page_session.h"
#include "./ui_page_session.h"

#include <QJsonDocument>
#include <QMessageBox>
#include <QMetaMethod>

PageSession::PageSession(QWidget *parent)
    : QWidget(parent), ui(new Ui::PageSession)
{
    ui->setupUi(this);
    ui->splitter_top->setStretchFactor(0, 2);
    ui->splitter_top->setStretchFactor(1, 3);
    ui->splitter_level1->setStretchFactor(0, 1);
    ui->splitter_level1->setStretchFactor(1, 2);

    connect_signals_slots();

    ZConfig config = ZConfig();
    showConfig(config);
}

PageSession::~PageSession()
{
    delete ui;
}

void PageSession::showConfig(ZConfig &zConfig)
{
    QJsonDocument json = QJsonDocument::fromJson(zConfig.getStr().toUtf8());
    QString j = "```\n" + QString(json.toJson(QJsonDocument::JsonFormat::Indented)) + "```";
    ui->jsonTextBrowser->setMarkdown(j);
}

QListWidgetItem *create_endpoint()
{
    auto item = new QListWidgetItem();
    item->setText("tcp/127.0.0.1:7447");
    auto font = QFont();
    font.setPixelSize(16);
    item->setFont(font);
    item->setFlags(
        Qt::ItemIsSelectable | Qt::ItemIsEditable | Qt::ItemIsUserCheckable | Qt::ItemIsEnabled
            | Qt::ItemNeverHasChildren
    );
    return item;
}

void PageSession::connectAdd_clicked(bool checked)
{
    auto item = create_endpoint();
    ui->connectListWidget->addItem(item);
}

void PageSession::connectDel_clicked(bool checked)
{
    auto row_index = ui->connectListWidget->currentRow();
    auto item = ui->connectListWidget->takeItem(row_index);
    delete item;
}

void PageSession::listenAdd_clicked(bool checked)
{
    auto item = create_endpoint();
    ui->listenListWidget->addItem(item);
}

void PageSession::listenDel_clicked(bool checked)
{
    auto row_index = ui->listenListWidget->currentRow();
    auto item = ui->listenListWidget->takeItem(row_index);
    delete item;
}

void PageSession::connect_signals_slots()
{
    connect(ui->update, &QPushButton::clicked, this, &PageSession::update_clicked);
    connect(ui->connectAdd, &QPushButton::clicked, this, &PageSession::connectAdd_clicked);
    connect(ui->connectDel, &QPushButton::clicked, this, &PageSession::connectDel_clicked);
    connect(ui->listenAdd, &QPushButton::clicked, this, &PageSession::listenAdd_clicked);
    connect(ui->listenDel, &QPushButton::clicked, this, &PageSession::listenDel_clicked);
    connect(ui->sessionPushButton, &QPushButton::clicked, this, &PageSession::sessionPushButton_clicked);
}

bool PageSession::setConnects(ZConfig &zConfig)
{
    QSet<QString> set;
    for (int i = 0; i < ui->connectListWidget->count(); i++) {
        QString text = ui->connectListWidget->item(i)->text();
        set.insert(text);
    }
    return zConfig.setConnects(set.values());
}

bool PageSession::setListens(ZConfig &zConfig)
{
    QSet<QString> set;
    for (int i = 0; i < ui->listenListWidget->count(); i++) {
        QString text = ui->listenListWidget->item(i)->text();
        set.insert(text);
    }
    return zConfig.setListens(set.values());
}

bool PageSession::setMode(ZConfig &zConfig)
{
    ZMode mode;
    QString value = ui->modeComboBox->currentText();
    if (value == "peer") {
        mode = ZMode::peer;
    }
    else {
        mode = ZMode::client;
    }

    return zConfig.setMode(mode);
}

void PageSession::update_clicked(bool checked)
{
    ZConfig config = ZConfig();
    if (!checkAndSetConfig(config)) {
        return;
    }
    QMessageBox msgBox;
    msgBox.setText(tr("参数设置成功"));
    msgBox.exec();

    showConfig(config);
}

bool PageSession::checkAndSetConfig(ZConfig &zConfig)
{
    QMessageBox msgBox;
    if (!setConnects(zConfig)) {
        msgBox.setText(tr("connect 参数错误"));
        msgBox.exec();
        return false;
    }

    if (!setListens(zConfig)) {
        msgBox.setText(tr("listen 参数错误"));
        msgBox.exec();
        return false;
    }

    if (!setMode(zConfig)) {
        msgBox.setText(tr("mode 参数错误"));
        msgBox.exec();
        return false;
    }
    return true;
}

ZConfig *PageSession::getZConfig()
{
    auto *config = new ZConfig();
    if (checkAndSetConfig(*config)) {
        return config;
    }
    else {
        delete config;
        return nullptr;
    }
}

void PageSession::sessionPushButton_clicked(bool check)
{
    if (!check) {
        emit sessionClose();
        qDebug() << "emit sessionClose";
        return;
    }

    auto config = getZConfig();
    if (config == nullptr) {
        ui->sessionPushButton->setChecked(false);
        return;
    }

    QMetaMethod signalStatus = QMetaMethod::fromSignal(&PageSession::sessionOpen);
    if (isSignalConnected(signalStatus)) {
        emit sessionOpen(config);
        qDebug() << "emit sessionOpen";
    }
    else {
        ui->sessionPushButton->setChecked(false);
        delete config;
    }
}

void PageSession::setSessionPushButtonChecked(bool b)
{
    ui->sessionPushButton->setChecked(b);
}


