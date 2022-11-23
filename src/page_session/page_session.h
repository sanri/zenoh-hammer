#pragma once

#include <QWidget>

QT_BEGIN_NAMESPACE
namespace Ui { class PageSession; }
QT_END_NAMESPACE

class PageSession:public QWidget
{
public:
    PageSession(QWidget *parent = nullptr);
    ~PageSession();

private:
    Ui::PageSession*ui;
};

