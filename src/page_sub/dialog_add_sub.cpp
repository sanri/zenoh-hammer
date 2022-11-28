#include "dialog_add_sub.h"
#include "ui_dialog_add_sub.h"


DialogAddSub::DialogAddSub(QString &name, QString &keyExpr, QWidget *parent)
    :
    QDialog(parent), ui(new Ui::DialogAddSub), name(name), keyExpr(keyExpr)
{
    ui->setupUi(this);
    connect_signals_slots();
}

DialogAddSub::~DialogAddSub()
{
    delete ui;
}

void DialogAddSub::connect_signals_slots()
{
    connect(ui->acceptPushButton, &QPushButton::clicked, this, &DialogAddSub::acceptPushButton_clicked);
    connect(ui->cancelPushButton, &QPushButton::clicked, this, &DialogAddSub::cancelPushButton_clicked);
}

void DialogAddSub::cancelPushButton_clicked(bool checked)
{
    this->done(-1);
}

void DialogAddSub::acceptPushButton_clicked(bool checked)
{
    name = ui->namelineEdit->text();
    keyExpr = ui->keyExprlineEdit->text();
    this->done(0);
}
