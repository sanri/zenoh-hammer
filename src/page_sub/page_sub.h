#pragma once

#include <QWidget>

QT_BEGIN_NAMESPACE
namespace Ui
{
class PageSub;
}
QT_END_NAMESPACE

class PageSub: public QWidget
{
Q_OBJECT

public:
    explicit PageSub(QWidget *parent = nullptr);
    ~PageSub() override;

private:
    Ui::PageSub *ui;
};


