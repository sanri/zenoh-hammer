#include <algorithm>
#include <sstream>
#include <chrono>
#include <QFileSystemModel>
#include <utility>
#include <iomanip>
#include <QMessageBox>
#include "page_sub.h"
#include "ui_page_sub.h"
#include "../page_mainwindow/mainwindow.h"


PageSub::PageSub(QWidget *parent)
    :
    QWidget(parent), ui(new Ui::PageSub)
{
    ui->setupUi(this);
    connect_signals_slots();
    ui->splitter_top->setStretchFactor(0, 1);
    ui->splitter_top->setStretchFactor(1, 4);
    ui->splitter_level1->setStretchFactor(0, 1);
    ui->splitter_level1->setStretchFactor(1, 3);
}

PageSub::~PageSub()
{
    delete ui;
}

void PageSub::clear_clicked(bool checked)
{
}

void PageSub::connect_signals_slots()
{
    connect(ui->clear, &QPushButton::clicked, this, &PageSub::clear_clicked);
    connect(ui->subAdd, &QPushButton::clicked, this, &PageSub::subAdd_clicked);
    connect(ui->subDel, &QPushButton::clicked, this, &PageSub::subDel_clicked);
    connect(ui->keyTreeView, &QTreeView::clicked, this, &PageSub::keyTreeView_clicked);
    connect(ui->subListWidget,&QListWidget::itemClicked,this,&PageSub::subListWidget_clicked);
}

SubData *PageSub::getSubData(QString name)
{
    auto it = map.find(name);
    if (it == map.end())
        return nullptr;
    return it.value();
}

void PageSub::keyTreeView_clicked(const QModelIndex &index)
{
    SubTreeModel *treeModelNow = (SubTreeModel *) ui->keyTreeView->model();
    if (treeModelNow == nullptr)
        return;
    QString path = treeModelNow->getPath(index);
    ui->selectKey->setText(path);
    QString name = treeModelNow->getName();
    SubData *subData = getSubData(name);
    if (subData == nullptr)
        return;
    SubTableModel *tableModel = subData->getTableModel(path);
    ui->valueTableView->setModel(tableModel);
    ui->valueTableView->setRootIndex(QModelIndex());
}

void PageSub::subListWidget_clicked(QListWidgetItem *item)
{
    QString name = item->text();
    SubData *data = map[name];
    ui->selectKeyExpr->setText(data->getKeyExpr());

    ui->keyTreeView->setModel(data->getTreeModel());
    ui->keyTreeView->setRootIndex(QModelIndex());

    ui->valueTableView->setModel(nullptr);
}

void PageSub::newSubMsg(QString name, const QSharedPointer<ZSample> sample)
{
    qDebug()<<"newSubMsg name: "<< name <<",  key: "<<sample->getKey();
    auto it = map.find(name);
    if (it == map.end())
        return;
    SubData *data = it.value();
    data->updateTreeModel(sample->getKey());
    data->updateTableModel(sample);
}

QListWidgetItem *create_subListWidget_item(QString name)
{
    auto item = new QListWidgetItem();
    item->setText(name);
    auto font = QFont();
    font.setPixelSize(16);
    item->setFont(font);
    item->setFlags(
        Qt::ItemIsSelectable | Qt::ItemIsUserCheckable | Qt::ItemIsEnabled
            | Qt::ItemNeverHasChildren
    );
    return item;
}

void PageSub::newSubscriberResult(QZSubscriber *subscriber)
{
    if (subscriber == nullptr) {
        QMessageBox msgBox;
        msgBox.setText(tr("注册新订阅失败"));
        msgBox.exec();
        return;
    }
    QString name = subscriber->getName();
    QString keyExpr = subscriber->getKeyExpr();

    auto *data = new SubData(name, keyExpr);
    map.insert(name, data);
    connect(subscriber, &QZSubscriber::newSubMsg, this, &PageSub::newSubMsg, Qt::ConnectionType::QueuedConnection);

    ui->subListWidget->addItem(create_subListWidget_item(name));
}

void PageSub::delSubscriberResult(QString name)
{
    SubData *subData = getSubData(name);
    if (subData == nullptr)return;

    SubTableModel* tableModel = subData->getTableModel(name);
    SubTreeModel* treeModel = subData->getTreeModel();

    if(ui->valueTableView->model() == tableModel){
        ui->valueTableView->setModel(nullptr);
    }
    if(ui->keyTreeView->model() == treeModel){
        ui->keyTreeView->setModel(nullptr);
    }

    delete subData;
    map.remove(name);

    auto row = ui->subListWidget->currentRow();
    auto item = ui->subListWidget->takeItem(row);
    ui->subListWidget->removeItemWidget(item);
}

void PageSub::subAdd_clicked(bool checked)
{
    QString name;
    QString keyExpr;
    auto dialog = DialogAddSub(name, keyExpr, this);
    int r = dialog.exec();
    if (r == 0) {
        // 检查 name 是否被注册
        if (map.contains(name)) {
            // 此名称已被使用
            QMessageBox msgBox;
            msgBox.setText(tr("name 已被使用! 请重新命名"));
            msgBox.exec();
            return;
        }

        emit newSubscriber(name, keyExpr);
    }
}

void PageSub::subDel_clicked(bool checked)
{
    auto item = ui->subListWidget->currentItem();
    QString name = item->text();
    emit delSubscriber(name);
}

SubTreeItem::SubTreeItem(QString key, bool isValue, SubTreeItem *parentItem)
    :
    key(std::move(key)), parent(parentItem), isValue(isValue)
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
    return (int) children.length();
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
        return parent->children.indexOf(const_cast<SubTreeItem *>(this));

    return 0;
}

SubTreeItem *SubTreeItem::parentItem()
{
    return parent;
}

SubTreeItem *SubTreeItem::findKey(QString &n)
{
    for (SubTreeItem *item: children) {
        if (item->key == n) {
            return item;
        }
    }
    return nullptr;
}

void SubTreeItem::sortChildren()
{
    std::sort(children.begin(), children.end(), [](SubTreeItem *a, SubTreeItem *b)
    {
        return (a->key < b->key);
    });
}

QString SubTreeItem::getKey()
{
    return key;
}

SubTreeModel::SubTreeModel(QString name, QObject *parent)
    :
    QAbstractItemModel(parent), name(name)
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

    SubTreeItem *item = static_cast<SubTreeItem *>(index.internalPointer());

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
        parentItem = static_cast<SubTreeItem *>(parent.internalPointer());

    SubTreeItem *childItem = parentItem->child(row);
    if (childItem)
        return createIndex(row, column, childItem);
    return QModelIndex();
}

QModelIndex SubTreeModel::parent(const QModelIndex &index) const
{
    if (!index.isValid())
        return QModelIndex();

    SubTreeItem *childItem = static_cast<SubTreeItem *>(index.internalPointer());
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
        parentItem = static_cast<SubTreeItem *>(parent.internalPointer());

    return parentItem->childCount();
}

int SubTreeModel::columnCount(const QModelIndex &parent) const
{
    if (parent.isValid())
        return static_cast<SubTreeItem *>(parent.internalPointer())->columnCount();
    return rootItem->columnCount();
}

bool SubTreeModel::addNewValueKey(QString &key)
{
    auto key_list = key.split(u'/');
    SubTreeItem *item = rootItem;
    for (int i = 0; i < key_list.count(); i++) {
        SubTreeItem *item_new = item->findKey(key_list[i]);
        if (item_new == nullptr) {
            // 判断是否为叶节点
            bool isValue = (i == (key_list.count() - 1));
            // 获得父节点 index
            QModelIndex idx = (item == rootItem) ? QModelIndex() : createIndex(item->row(), 0, item);

            // 开始更新
            item_new = new SubTreeItem(key_list[i], isValue, item);
            int end_row = item->childCount() - 1;
            if (end_row < 0)end_row = 0;
            beginInsertRows(idx, 0, end_row);
            item->appendChild(item_new);
            item->sortChildren();
            endInsertRows();

            if (isValue) {
                return true;
            }
        }
        item = item_new;
    }

    return false;
}

QString SubTreeModel::getPath(const QModelIndex &index)
{
    SubTreeItem *item = static_cast<SubTreeItem *>(index.internalPointer());
    QQueue<QString> queue;
    while (item != rootItem) {
        queue.push_front(item->getKey());
        item = item->parentItem();
    }
    return queue.join(u'/');
}

QString SubTreeModel::getName() const
{
    return name;
}

SubDataItem::SubDataItem(ZSample &sample)
    :
    timestamp(sample.timestamp), encoding(sample.encoding), payload(std::move(sample.payload))
{
    timeNow = std::chrono::system_clock::now();
}

SubDataItem::SubDataItem(int i)
{
    payload = QString::number(i).toUtf8();
    timeNow = std::chrono::system_clock::now();
    encoding = Z_ENCODING_PREFIX_APP_INTEGER;
    timestamp = ZTimestamp(1000, 123);
}

SubDataItem::SubDataItem(double f)
{
    payload = QString::number(f).toUtf8();
    timeNow = std::chrono::system_clock::now();
    encoding = Z_ENCODING_PREFIX_APP_FLOAT;
    timestamp = ZTimestamp(2000, 321);

}

SubDataItem::SubDataItem(QString s)
{
    payload = s.toUtf8();
    timeNow = std::chrono::system_clock::now();
    encoding = Z_ENCODING_PREFIX_TEXT_PLAIN;
    timestamp = ZTimestamp(3000, 567);
}

QVariant SubDataItem::get(int index) const
{
    switch (index) {
    case 0:return payloadToV();
    case 1:return encodingToV();
    case 2:return timestampToV();
    case 3:return timeNowToV();
    default:return {};
    }
}

int SubDataItem::column()
{
    return 4;
}

QVariant SubDataItem::payloadToV() const
{
    switch (encoding) {
    case Z_ENCODING_PREFIX_EMPTY:return {};
    case Z_ENCODING_PREFIX_APP_OCTET_STREAM:break;
    case Z_ENCODING_PREFIX_APP_CUSTOM:break;
    case Z_ENCODING_PREFIX_TEXT_PLAIN:
        if (payload.length() < 80)
            return {QString(payload)};
        break;
    case Z_ENCODING_PREFIX_APP_PROPERTIES:break;
    case Z_ENCODING_PREFIX_APP_JSON:
        if (payload.length() < 80)
            return {QString(payload)};
        break;
    case Z_ENCODING_PREFIX_APP_SQL:break;
    case Z_ENCODING_PREFIX_APP_INTEGER:return {QString(payload)};
    case Z_ENCODING_PREFIX_APP_FLOAT:return {QString(payload)};
    case Z_ENCODING_PREFIX_APP_XML:break;
    case Z_ENCODING_PREFIX_APP_XHTML_XML:break;
    case Z_ENCODING_PREFIX_APP_X_WWW_FORM_URLENCODED:break;
    case Z_ENCODING_PREFIX_TEXT_JSON:
        if (payload.length() < 80)
            return {QString(payload)};
        break;
    case Z_ENCODING_PREFIX_TEXT_HTML:break;
    case Z_ENCODING_PREFIX_TEXT_XML:break;
    case Z_ENCODING_PREFIX_TEXT_CSS:break;
    case Z_ENCODING_PREFIX_TEXT_CSV:break;
    case Z_ENCODING_PREFIX_TEXT_JAVASCRIPT:break;
    case Z_ENCODING_PREFIX_IMAGE_JPEG:break;
    case Z_ENCODING_PREFIX_IMAGE_PNG:break;
    case Z_ENCODING_PREFIX_IMAGE_GIF:break;
    default: return {};
    }
    return {"..."};
}

QVariant SubDataItem::timestampToV() const
{
    return {timestamp.format()};
}

QVariant SubDataItem::encodingToV() const
{
    switch (encoding) {
    case Z_ENCODING_PREFIX_EMPTY:return {};
    case Z_ENCODING_PREFIX_APP_OCTET_STREAM:return {"app_octet_stream"};
    case Z_ENCODING_PREFIX_APP_CUSTOM:return {"app_custom"};
    case Z_ENCODING_PREFIX_TEXT_PLAIN:return {"text_plain"};
    case Z_ENCODING_PREFIX_APP_PROPERTIES:return {"app_properties"};
    case Z_ENCODING_PREFIX_APP_JSON:return {"app_json"};
    case Z_ENCODING_PREFIX_APP_SQL:return {"app_sql"};
    case Z_ENCODING_PREFIX_APP_INTEGER:return {"app_integer"};
    case Z_ENCODING_PREFIX_APP_FLOAT:return {"app_float"};
    case Z_ENCODING_PREFIX_APP_XML:return {"app_xml"};
    case Z_ENCODING_PREFIX_APP_XHTML_XML:return {"app_xhtml_xml"};
    case Z_ENCODING_PREFIX_APP_X_WWW_FORM_URLENCODED:return {"app_x_www_form_urlencoded"};
    case Z_ENCODING_PREFIX_TEXT_JSON:return {"text_json"};
    case Z_ENCODING_PREFIX_TEXT_HTML:return {"text_html"};
    case Z_ENCODING_PREFIX_TEXT_XML:return {"text_xml"};
    case Z_ENCODING_PREFIX_TEXT_CSS:return {"text_css"};
    case Z_ENCODING_PREFIX_TEXT_CSV:return {"text_csv"};
    case Z_ENCODING_PREFIX_TEXT_JAVASCRIPT:return {"text_javascript"};
    case Z_ENCODING_PREFIX_IMAGE_JPEG:return {"image_jpeg"};
    case Z_ENCODING_PREFIX_IMAGE_PNG:return {"image_png"};
    case Z_ENCODING_PREFIX_IMAGE_GIF:return {"image_gif"};
    default:return {};
    }
}

QVariant SubDataItem::timeNowToV() const
{
    std::time_t t_c = std::chrono::system_clock::to_time_t(timeNow);
    std::ostringstream o;
    o << std::put_time(std::localtime(&t_c), "%F %T.");
    auto ms = std::chrono::duration_cast<std::chrono::milliseconds>(timeNow.time_since_epoch());
    int milltime = ms.count() % 1000;
    QString datetime = QString(o.str().c_str()) + QString::number(milltime);
    return {datetime};
}

SubTableModel::SubTableModel(QObject *parent)
    : QAbstractTableModel(parent)
{}

SubTableModel::~SubTableModel()
{
    qDeleteAll(queue);
}

int SubTableModel::rowCount(const QModelIndex &parent) const
{
    return (int) queue.count();
}

int SubTableModel::columnCount(const QModelIndex &parent) const
{
    return SubDataItem::column();
}

QVariant SubTableModel::data(const QModelIndex &index, int role) const
{
    if (role != Qt::DisplayRole)
        return {};

    int row = index.row();
    if ((row < 0) || (row >= queue.count()))
        return {};
    SubDataItem *item = queue[row];

    return item->get(index.column());
}

QVariant SubTableModel::headerData(int section, Qt::Orientation orientation, int role) const
{
    if (orientation == Qt::Horizontal && role == Qt::DisplayRole) {
        if (section == 0)
            return {tr("值")};
        else if (section == 1)
            return {tr("类型")};
        else if (section == 2)
            return {tr("Zenoh时间戳")};
        else if (section == 3)
            return {tr("本机时间戳")};
        else
            return {};
    }
    return {};
}

void SubTableModel::addData(SubDataItem *data)
{
    int first_row = (int)queue.length() - 1;
    if (first_row < 0) first_row = 0;
    int last_row = first_row;
    beginInsertRows(QModelIndex(), first_row, last_row);
    queue.push_back(data);
    endInsertRows();
}

SubData::SubData(QString name, QString keyExpr)
    :
    name(name), keyExpr(std::move(keyExpr)), treeModel(new SubTreeModel(name))
{
}

SubData::~SubData()
{
    qDeleteAll(map);
    delete treeModel;
}

QString SubData::getName()
{
    return name;
}

QString SubData::getKeyExpr()
{
    return keyExpr;
}

SubTreeModel *SubData::getTreeModel()
{
    return treeModel;
}

SubTableModel *SubData::getTableModel(QString key)
{
    auto it = map.find(key);
    if (it == map.end())
        return nullptr;
    else
        return it.value();
}

void SubData::updateTreeModel(QString key)
{
    treeModel->addNewValueKey(key);
}

void SubData::updateTableModel(const QSharedPointer<ZSample> sample)
{
    QString key = sample->getKey();
    auto it = map.find(key);
    SubTableModel *model;
    if (it == map.end()) {
        model = new SubTableModel();
        map.insert(key, model);
    }
    else {
        model = it.value();
    }

    auto data = new SubDataItem(*sample);
    model->addData(data);
}
