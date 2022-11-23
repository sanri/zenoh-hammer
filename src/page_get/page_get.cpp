//
// Created by 唐晶 on 2022/11/23.
//

// You may need to build the project (run Qt uic code generator) to get "ui_page_get.h" resolved

#include "page_get.h"
#include "ui_page_get.h"


PageGet::PageGet(QWidget *parent)
    :
    QWidget(parent), ui(new Ui::PageGet)
{
    ui->setupUi(this);
}

PageGet::~PageGet()
{
    delete ui;
}
