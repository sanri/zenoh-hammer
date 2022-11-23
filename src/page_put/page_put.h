//
// Created by 唐晶 on 2022/11/23.
//

#ifndef PAGE_PUT_H
#define PAGE_PUT_H

#include <QWidget>


QT_BEGIN_NAMESPACE
namespace Ui
{
class PagePut;
}
QT_END_NAMESPACE

class PagePut: public QWidget
{
Q_OBJECT

public:
    explicit PagePut(QWidget *parent = nullptr);
    ~PagePut() override;

private:
    Ui::PagePut *ui;
};


#endif //PAGE_PUT_H
