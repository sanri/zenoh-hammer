//
// Created by 唐晶 on 2022/11/23.
//

// You may need to build the project (run Qt uic code generator) to get "ui_page_pub.h" resolved

#include "page_pub.h"
#include "ui_page_pub.h"


PagePub::PagePub(QWidget *parent)
    :
    QWidget(parent), ui(new Ui::PagePub)
{
    ui->setupUi(this);
}

PagePub::~PagePub()
{
    delete ui;
}
