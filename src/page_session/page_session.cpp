#include "page_session.h"
#include "./ui_page_session.h"
PageSession::PageSession(QWidget *parent): QWidget(parent),ui(new Ui::PageSession)
{
    ui->setupUi(this);
}
PageSession::~PageSession()
{
    delete ui;
}
