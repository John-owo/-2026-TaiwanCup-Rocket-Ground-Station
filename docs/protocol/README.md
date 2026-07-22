# Protocol 規格與共同測試向量

此目錄保存 Ground Station 與空中端共用的 Protocol v1／v2 規格快照、golden vectors 及純 Python 驗證工具，讓 README 的規格連結不依賴其他 repository。

## 文件

- [Protocol v1 完整規格](PROTOCOL_V1.md)
- [Protocol v2 完整規格](PROTOCOL_V2.md)
- [Protocol v1 golden vectors](test_vectors_v1.json)
- [Protocol v2 golden vectors](test_vectors_v2.json)

## 驗證

從 Ground Station repository 根目錄執行：

```powershell
python .\docs\protocol\verify_test_vectors.py
python .\docs\protocol\verify_test_vectors_v2.py
```

兩個驗證器只使用 Python standard library。任何 protocol encoder、decoder、CRC、欄位長度或 duplicate semantics 變更，都必須同步更新兩端實作與對應 vectors。
