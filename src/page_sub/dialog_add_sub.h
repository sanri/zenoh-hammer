#pragma once

#include <QDialog>


QT_BEGIN_NAMESPACE
namespace Ui
{
class DialogAddSub;
}
QT_END_NAMESPACE

class DialogAddSub: public QDialog
{
Q_OBJECT

public:
    explicit DialogAddSub(QString &name, QString &keyExpr, QWidget *parent = nullptr);
    ~DialogAddSub() override;

public slots:
    void acceptPushButton_clicked(bool checked);
    void cancelPushButton_clicked(bool checked);

private:
    void connect_signals_slots();

private:
    Ui::DialogAddSub *ui;
    QString &name;
    QString &keyExpr;
};


