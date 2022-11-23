//
// Created by 唐晶 on 2022/11/23.
//

// You may need to build the project (run Qt uic code generator) to get "ui_page_put.h" resolved

#include "page_put.h"
#include "ui_page_put.h"


PagePut::PagePut(QWidget *parent)
    :
    QWidget(parent), ui(new Ui::PagePut)
{
    ui->setupUi(this);
}

PagePut::~PagePut()
{
    delete ui;
}
