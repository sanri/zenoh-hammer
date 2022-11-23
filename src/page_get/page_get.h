//
// Created by 唐晶 on 2022/11/23.
//

#ifndef PAGE_GET_H
#define PAGE_GET_H

#include <QWidget>


QT_BEGIN_NAMESPACE
namespace Ui
{
class PageGet;
}
QT_END_NAMESPACE

class PageGet: public QWidget
{
Q_OBJECT

public:
    explicit PageGet(QWidget *parent = nullptr);
    ~PageGet() override;

private:
    Ui::PageGet *ui;
};


#endif //PAGE_GET_H
