#pragma once

#include <QWidget>
#include <QAbstractListModel>
#include <QModelIndex>
#include <QListWidget>
#include <QQueue>
#include "src/qzenoh/qzenoh.h"
#include "dialog_add_sub.h"

QT_BEGIN_NAMESPACE
namespace Ui
{
class PageSub;
}
QT_END_NAMESPACE

class SubDataItem;
class SubData;
class SubDataList;

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
    explicit SubTreeModel(QString name, QObject *parent = nullptr);
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
    QString getName() const;

private:
    const QString name;
    SubTreeItem *rootItem;
};

// 表结构
// 值 | 类型 | Zenoh时间戳 | 本机时间戳
class SubDataItem
{
public:
    // 会消耗掉 sample
    explicit SubDataItem(ZSample &sample);
    explicit SubDataItem(int i);
    explicit SubDataItem(double f);
    explicit SubDataItem(QString s);
    ~SubDataItem() = default;

    QVariant get(int index) const;
    static int column();

private:
    QVariant payloadToV() const;
    QVariant timestampToV() const;
    QVariant encodingToV() const;
    QVariant timeNowToV() const;

private:
    QByteArray payload;
    ZTimestamp timestamp;
    std::chrono::time_point<std::chrono::system_clock> timeNow;
    z_encoding_prefix_t encoding;
};

// 表结构
// 值 | 类型 | Zenoh时间戳 | 本机时间戳
class SubTableModel: public QAbstractTableModel
{
Q_OBJECT

public:
    explicit SubTableModel(QObject *parent = nullptr);
    ~SubTableModel();

    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    int columnCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role) const override;
    QVariant headerData(int section, Qt::Orientation orientation,
                        int role = Qt::DisplayRole) const override;
    void addData(SubDataItem *data);

private:
    QQueue<SubDataItem *> queue;
};

class SubData
{
public:
    SubData(QString name, QString keyExpr);
    ~SubData();
    QString getName();
    QString getKeyExpr();
    SubTreeModel *getTreeModel();
    SubTableModel *getTableModel(QString key);
    void updateTreeModel(QString key);
    void updateTableModel(const QSharedPointer<ZSample> sample);

private:
    const QString name;
    const QString keyExpr;
    // key
    QMap<QString, SubTableModel *> map;
    SubTreeModel *treeModel;
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
    void subListWidget_clicked(QListWidgetItem *item);
    void subAdd_clicked(bool checked);
    void subDel_clicked(bool checked);
    void newSubMsg(QString name, const QSharedPointer<ZSample> sample);

    // 如果注册失败, 返回空指针
    void newSubscriberResult(QZSubscriber *subscriber);
    void delSubscriberResult(QString name);

signals:
    void newSubscriber(QString name, QString keyExpr);
    void delSubscriber(QString name);

private:
    void connect_signals_slots();
    SubData *getSubData(QString name);

private:
    Ui::PageSub *ui;
    // name
    QMap<QString, SubData *> map;
};


