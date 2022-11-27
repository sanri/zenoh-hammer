#pragma once

#include <QWidget>
#include <QAbstractListModel>
#include <QModelIndex>
#include <QQueue>
#include "src/qzenoh/qzenoh.h"

QT_BEGIN_NAMESPACE
namespace Ui
{
class PageSub;
}
QT_END_NAMESPACE

class SubDataValue;
class SubData;

class StringListModel: public QAbstractListModel
{
Q_OBJECT

public:
    explicit StringListModel(const QStringList &strings, QObject *parent = nullptr)
        : QAbstractListModel(parent), stringList(strings)
    {}

    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role) const override;
    QVariant headerData(int section, Qt::Orientation orientation,
                        int role = Qt::DisplayRole) const override;

    Qt::ItemFlags flags(const QModelIndex &index) const override;
    bool setData(const QModelIndex &index, const QVariant &value,
                 int role = Qt::EditRole) override;

private:
    QStringList stringList;
};

class SubTreeItem
{
public:
    explicit SubTreeItem(QString key, bool isValue, SubTreeItem *parentItem = nullptr);
    ~SubTreeItem();

    void appendChild(SubTreeItem *child);

    SubTreeItem *child(int row);
    int childCount() const;
    int columnCount() const;
    QVariant data(int column) const;
    int row() const;
    SubTreeItem *parentItem();

    SubTreeItem *findKey(QString &n);
    void sortChildren();
    QString getKey();

private:
    QList<SubTreeItem *> children;
    QString key;
    bool isValue;
    SubTreeItem *parent;
};

class SubTreeModel: public QAbstractItemModel
{
Q_OBJECT

public:
    explicit SubTreeModel(QObject *parent = nullptr);
    ~SubTreeModel();

    QVariant data(const QModelIndex &index, int role) const override;
    Qt::ItemFlags flags(const QModelIndex &index) const override;
    QVariant headerData(int section, Qt::Orientation orientation,
                        int role = Qt::DisplayRole) const override;
    QModelIndex index(int row, int column,
                      const QModelIndex &parent = QModelIndex()) const override;
    QModelIndex parent(const QModelIndex &index) const override;
    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    int columnCount(const QModelIndex &parent = QModelIndex()) const override;

    // 增加新的变量到模型中
    // 返回 false: 表示此变量路径已在模型中, 不更新模型
    //     true:  表示此变量路径为新路径, 更新模型
    bool addNewValueKey(QString &key);

    // 返回完整路径
    QString getPath(const QModelIndex &index);

private:
    SubTreeItem *rootItem;
};

class SubDataValue
{
public:
    // 会消耗掉 sample
    explicit SubDataValue(ZSample &sample);
    ~SubDataValue() = default;

private:
    QByteArray payload;
    ZTimestamp timestamp;
    z_encoding_prefix_t encoding;
};

class SubDataList
{
public:
    SubDataList();
    ~SubDataList();

private:
    QQueue<SubDataValue *> list;
};

class SubData
{
public:
    SubData() = default;
    ~SubData() = default;

private:
    QString name;
    QString keyExpr;
    QMap<QString, SubDataList *> list;
};

class PageSub: public QWidget
{
Q_OBJECT

public:
    explicit PageSub(QWidget *parent = nullptr);
    ~PageSub() override;

public slots:
    void clear_clicked(bool checked);
    void keyTreeView_clicked(const QModelIndex &index);

private:
    void connect_signals_slots();

private:
    Ui::PageSub *ui;
    SubTreeModel *treeModel = nullptr;
};


