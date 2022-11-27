//
// Created by 唐晶 on 2022/11/23.
//

// You may need to build the project (run Qt uic code generator) to get "ui_page_sub.h" resolved

#include <QFileSystemModel>
#include <utility>
#include "page_sub.h"
#include "ui_page_sub.h"


PageSub::PageSub(QWidget *parent)
    :
    QWidget(parent), ui(new Ui::PageSub)
{
    ui->setupUi(this);
    ui->splitter_top->setStretchFactor(0, 1);
    ui->splitter_top->setStretchFactor(1, 4);
    ui->splitter_level1->setStretchFactor(0,1);
    ui->splitter_level1->setStretchFactor(1,2);

    QStringList list = QStringList {"abc","def","ghi","jkl","mno"};
    auto mode = new StringListModel(list);
    ui->valueTableView->setModel(mode);
    ui->valueTableView->setRootIndex(QModelIndex());

}

PageSub::~PageSub()
{
    delete ui;
}

int StringListModel::rowCount(const QModelIndex &parent) const
{
    return stringList.count();
}

QVariant StringListModel::data(const QModelIndex &index, int role) const
{
    if (!index.isValid())
        return QVariant();

    if (index.row() >= stringList.size())
        return QVariant();

    if (role == Qt::DisplayRole || role == Qt::EditRole)
        return stringList.at(index.row());
    else
        return QVariant();
}

QVariant StringListModel::headerData(int section, Qt::Orientation orientation, int role) const
{
    if (role != Qt::DisplayRole)
        return QVariant();

    if (orientation == Qt::Horizontal)
        return QStringLiteral("Column %1").arg(section);
    else
//        return QVariant();
        return QStringLiteral("%1").arg(section);
}

Qt::ItemFlags StringListModel::flags(const QModelIndex &index) const
{
    if (!index.isValid())
        return Qt::ItemIsEnabled;

    return QAbstractListModel::flags(index) | Qt::ItemIsEditable;
}

bool StringListModel::setData(const QModelIndex &index, const QVariant &value, int role)
{
    if (index.isValid() && role == Qt::EditRole) {
        stringList.replace(index.row(), value.toString());
        emit dataChanged(index, index, {role});
        return true;
    }
    return false;
}
SubTreeItem::SubTreeItem(QString  key,bool isValue, SubTreeItem *parentItem):
    key(std::move(key)), parent(parentItem),isValue(isValue)
{

}

SubTreeItem::~SubTreeItem()
{
    qDeleteAll(children);
}

void SubTreeItem::appendChild(SubTreeItem *child)
{
    children.append(child);
}

SubTreeItem *SubTreeItem::child(int row)
{
    if (row < 0 || row >= children.size())
        return nullptr;
    return children.at(row);
}

int SubTreeItem::childCount() const
{
    return children.count();
}

int SubTreeItem::columnCount() const
{
    return 2;
}

QVariant SubTreeItem::data(int column) const
{
    if (column == 0) {
        return QVariant(key);
    }
    else if (column == 1) {
        return isValue ? QVariant("*") : QVariant();
    }
    else {
        return QVariant();
    }
}

int SubTreeItem::row() const
{
    if (parent)
        return parent->children.indexOf(const_cast<SubTreeItem*>(this));

    return 0;
}

SubTreeItem *SubTreeItem::parentItem()
{
    return parent;
}

SubTreeItem *SubTreeItem::findKey(QString &n)
{
    for(SubTreeItem*item:children){
        if (item->key == n){
            return item;
        }
    }
    return nullptr;
}

SubTreeModel::SubTreeModel(QObject *parent):
    QAbstractItemModel(parent)
{
    rootItem = new SubTreeItem("", false);
}

SubTreeModel::~SubTreeModel()
{
    delete rootItem;
}

QVariant SubTreeModel::data(const QModelIndex &index, int role) const
{
    if (!index.isValid())
        return QVariant();

    if (role != Qt::DisplayRole)
        return QVariant();

    SubTreeItem *item = static_cast<SubTreeItem*>(index.internalPointer());

    return item->data(index.column());
}

Qt::ItemFlags SubTreeModel::flags(const QModelIndex &index) const
{
    if (!index.isValid())
        return Qt::NoItemFlags;

    return QAbstractItemModel::flags(index);
}

QVariant SubTreeModel::headerData(int section, Qt::Orientation orientation, int role) const
{
    if (orientation == Qt::Horizontal && role == Qt::DisplayRole) {
        if (section == 0)
            return QVariant(tr("路径"));
        else if (section == 1)
            return QVariant(tr("变量"));
        else
            return QVariant();
    }
    return QVariant();
}

QModelIndex SubTreeModel::index(int row, int column, const QModelIndex &parent) const
{
    if (!hasIndex(row, column, parent))
        return QModelIndex();

    SubTreeItem *parentItem;

    if (!parent.isValid())
        parentItem = rootItem;
    else
        parentItem = static_cast<SubTreeItem*>(parent.internalPointer());

    SubTreeItem *childItem = parentItem->child(row);
    if (childItem)
        return createIndex(row, column, childItem);
    return QModelIndex();
}

QModelIndex SubTreeModel::parent(const QModelIndex &index) const
{
    if (!index.isValid())
        return QModelIndex();

    SubTreeItem *childItem = static_cast<SubTreeItem*>(index.internalPointer());
    SubTreeItem *parentItem = childItem->parentItem();

    if (parentItem == rootItem)
        return QModelIndex();

    return createIndex(parentItem->row(), 0, parentItem);
}

int SubTreeModel::rowCount(const QModelIndex &parent) const
{
    SubTreeItem *parentItem;
    if (parent.column() > 0)
        return 0;

    if (!parent.isValid())
        parentItem = rootItem;
    else
        parentItem = static_cast<SubTreeItem*>(parent.internalPointer());

    return parentItem->childCount();
}

int SubTreeModel::columnCount(const QModelIndex &parent) const
{
    if (parent.isValid())
        return static_cast<SubTreeItem*>(parent.internalPointer())->columnCount();
    return rootItem->columnCount();
}

bool SubTreeModel::addNewValueKey(QString &key)
{
    auto key_list = key.split(u'/');
    SubTreeItem *item = rootItem;
    for (QString n: key_list) {
        SubTreeItem *item_new = item->findKey(n);
        if (item_new == nullptr) {

        }
    }

    return false;
}

SubDataValue::SubDataValue(ZSample &sample):
    timestamp(sample.timestamp),encoding(sample.encoding),payload(std::move(sample.payload))
{

}

SubDataList::SubDataList()
{

}

SubDataList::~SubDataList()
{
    qDeleteAll(list);
}
