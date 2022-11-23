//
// Created by 唐晶 on 2022/11/23.
//

#ifndef PAGE_PUB_H
#define PAGE_PUB_H

#include <QWidget>


QT_BEGIN_NAMESPACE
namespace Ui
{
class PagePub;
}
QT_END_NAMESPACE

class PagePub: public QWidget
{
Q_OBJECT

public:
    explicit PagePub(QWidget *parent = nullptr);
    ~PagePub() override;

private:
    Ui::PagePub *ui;
};


#endif //PAGE_PUB_H
