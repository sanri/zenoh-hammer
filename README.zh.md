# Zenoh-hammer

Zenoh的图形界面工具.    
方便进行简单的zenoh网络通信测试.

提供的功能类似于 zenoh 命令行工具 z_sub, z_put, z_get.


## 示例

<img src="media/example.gif">


## 功能
- [x] 支持发送、接收、查看文本类型数据
- [x] 支持发送、接收、查看png、jpeg格式图片数据
- [x] 可用十六进制查看器查看消息内容(目前只能查看消息的前5KB数据)
    - [ ] 十六进制查看器支持查看100MB内的数据
- [x] 可将软件界面内的配置数据保存为文件
- [x] 统计收到的订阅数据的频率
- [x] 支持中文、英文
    - [ ] 界面显示语言支持运行时切换 
- [ ] 可加载zenoh通信配置文件 


## 编译

克隆仓库后, 直接在项目主目录下运行命令

```shell
cargo build --release
```
