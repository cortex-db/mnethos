# mnethos для Windows — установка и настройка PATH

`mnethos.exe` — это собранный release-бинарник CLI-агента mnethos для Windows
(x86_64, target `x86_64-pc-windows-msvc`).

| | |
|---|---|
| Файл | `mnethos.exe` |
| Платформа | Windows 10/11, 64-bit (x64) |
| Размер | ~45.4 MB |
| SHA-256 | `78683b1bf2103304b917507859c10bab676fbd82873a8963cf0fd259ef919bc2` |

Цель настройки — чтобы команда `mnethos` запускалась из **любой** директории в
любом терминале (PowerShell, cmd, Windows Terminal). Для этого бинарник кладётся
в постоянную папку, а эта папка добавляется в пользовательский `Path`.

---

## Быстрая установка (PowerShell, рекомендуется)

Открой **обычный** PowerShell (права администратора НЕ нужны — меняем PATH
только для текущего пользователя), перейди в папку, где лежит `mnethos.exe`, и
выполни блок целиком:

```powershell
# 1. Папка для бинарника (на user PATH, не требует админа)
$dir = "$env:USERPROFILE\.local\bin"
New-Item -ItemType Directory -Force -Path $dir | Out-Null

# 2. Копируем exe туда
Copy-Item .\mnethos.exe $dir -Force

# 3. Снимаем «метку из интернета», если она есть (иначе SmartScreen ругается)
Unblock-File "$dir\mnethos.exe"

# 4. Добавляем папку в пользовательский Path НАВСЕГДА (если её там ещё нет)
$old = [Environment]::GetEnvironmentVariable("Path", "User")
if ($old -notlike "*$dir*") {
    [Environment]::SetEnvironmentVariable("Path", "$old;$dir", "User")
    Write-Host "Добавил $dir в PATH"
} else {
    Write-Host "$dir уже в PATH"
}
```

После этого **закрой и открой терминал заново** (изменения PATH подхватываются
только новыми процессами) и проверь:

```powershell
mnethos --version
```

Если выводится версия — готово, команда работает из любой директории.

---

## Ручная установка (через GUI, без PowerShell)

1. Создай папку, например `C:\Users\<твой-юзер>\.local\bin`
   (или `C:\tools\mnethos`).
2. Скопируй туда `mnethos.exe`.
3. Правый клик по файлу → **Свойства** → внизу поставь галочку
   **«Разблокировать» (Unblock)** → ОК. (Снимает предупреждение SmartScreen.)
4. Нажми `Win`, набери **«Изменение переменных среды для вашей учётной записи»**
   (Edit environment variables for your account) и открой.
5. В разделе **«Переменные среды пользователя»** выбери `Path` → **Изменить** →
   **Создать** → впиши путь к папке из шага 1 → ОК → ОК.
6. Открой **новый** терминал и проверь: `mnethos --version`.

---

## Обновление бинарника (важный нюанс Windows)

Если в момент обновления у тебя **запущена** интерактивная сессия `mnethos`,
Windows держит `.exe` заблокированным и просто перезаписать его не даст.
Запущенный exe можно **переименовать**, но не перезаписать — поэтому делаем
swap «через переименование»:

```powershell
$dst = "$env:USERPROFILE\.local\bin\mnethos.exe"
if (Test-Path $dst) {
    Rename-Item $dst "mnethos.old.$(Get-Date -Format yyyyMMddHHmmss).exe"
}
Copy-Item .\mnethos.exe $dst -Force
```

Новая версия подхватится при следующем запуске. Старые `mnethos.old.*.exe`
можно удалить позже, когда сессия закрыта.

---

## Если что-то не работает

- **`mnethos` не распознаётся как команда** — ты не открыл новый терминал после
  изменения PATH, либо папка не добавилась. Проверь:
  `[Environment]::GetEnvironmentVariable("Path","User")`.
- **SmartScreen / Defender блокирует запуск** — бинарник не подписан. Выполни
  `Unblock-File` (см. выше) или в окне SmartScreen нажми
  «Подробнее → Выполнить в любом случае».
- **Проверка целостности файла** — сверь хэш:
  ```powershell
  (Get-FileHash .\mnethos.exe -Algorithm SHA256).Hash
  ```
  должен совпасть с SHA-256 из таблицы выше.
