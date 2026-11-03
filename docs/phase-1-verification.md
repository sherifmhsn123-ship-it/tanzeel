# سجل تحقق المرحلة 1 — Issue #5

تاريخ التحقق: 2026-11-01 إلى 2026-11-03

هذا السجل يصف ما جرى اختباره محليًا على Windows قبل الانتقال إلى المرحلة 2. لا يثبت اكتمال بوابة المرحلة بذاته؛ يلزم مراجعة المالك ودمج PR #8.

## إصلاح حفظ اللغة

- اختبار `language-switch.test.tsx` يحاكي فشل التخزين، ويتحقق من عودة الواجهة إلى اللغة السابقة ومن ظهور رسالة خطأ.
- وفق سجل التشغيل، يفشل الاختبار على نسخة `App.tsx` السابقة لعدم وجود تنبيه خطأ (`Unable to find role="alert"`)، وينجح على التغيير الحالي.
- شُغّل الاختبار على التغيير الحالي ضمن مجموعة Vitest: نجحت الاختبارات الثلاثة في الملف.

## اختبار Tauri وSQLite

- `cargo test --workspace --all-targets` نجح، بما فيه اختبارا IPC في `tauri_ipc.rs`.
- اختبار IPC يستخدم SQLite في الذاكرة، ويختبر القراءة الافتراضية، وحفظ الإعدادات وقراءتها عبر `get_settings` و`save_settings`، ورفض مسار نسبي.
- اقتصر اختبار IPC الآلي على SQLite في الذاكرة، بينما غطى تشغيل النافذة الحقيقي قاعدة بيانات التطبيق الفعلية كما هو موضح أدناه.

## نافذة Windows ومراجعة RTL/LTR

- شُغّلت نافذة Tauri حقيقية عبر `corepack pnpm tauri dev` على Windows 11 Pro (10.0.26300). العملية `tanzeel-desktop`، والنافذة بعنوان `Tanzeel`، وصنفها `Tauri Window`.
- بدأ التطبيق بالعربية RTL، ثم بدّلت اللغة إلى الإنجليزية LTR من زر الرأس. رُصد التبديل عبر UI Automation بفارق واضح في مواقع الأزرار: في RTL كانت الرئيسية في أقصى اليمين، وفي LTR انتقلت إلى اليسار.
- كُتب الاختيار في قاعدة بيانات التطبيق الفعلية `%APPDATA%\app.tanzeel.desktop\tanzeel.db`؛ أظهر الاستعلام `language = en`. أُعيدت القيمة إلى `ar` بعد انتهاء التحقق.
- حُفظت لقطتان من نافذة Tauri الحقيقية: [phase-1-tauri-rtl.png](screenshots/phase-1-tauri-rtl.png) و[phase-1-tauri-ltr.png](screenshots/phase-1-tauri-ltr.png). اللقطتان القديمتان `phase-1-rtl.png` و`phase-1-ltr.png` من معاينة الويب في PR #4 وليستا دليلًا على نافذة Tauri.
- التحقق أعلاه آلي عبر UI Automation للنص والاتجاه والمواقع. المراجعة البصرية البشرية النهائية للقطتين مسؤولية المالك؛ لم تُسجَّل هنا كمراجعة بشرية مستقلة.
- لم يُعد إنتاج بناء Release منفصل بمعرّف تطبيق آخر وSQLite في الذاكرة في هذه الجولة. أي سجل سابق بذلك يحتاج أمر بناء وملف إعداد محفوظين حتى يكون قابلًا للتكرار.
- لم يُختبر المثبّت، ولا الإصدارات النظيفة على Windows 10 وWindows 11.

## نتائج الفحوصات المحلية

نجحت الأوامر التالية على Windows في 2026-11-02:

- `cargo fmt --all -- --check` — نجح.
- `cargo clippy --workspace --all-targets -- -D warnings` — نجح.
- `cargo test --workspace --all-targets` — 19 اختبارًا ناجحًا.
- `cargo build --workspace` — نجح.
- `corepack pnpm lint` — نجح.
- `corepack pnpm typecheck` — نجح.
- `corepack pnpm test` — 8 ملفات و21 اختبارًا ناجحًا، بما فيها العزل عن `fetch` أثناء العرض.
- `corepack pnpm test:i18n` — 3 اختبارات ناجحة.
- `corepack pnpm build` — نجح.

احتاج Corepack إلى مجلد cache محلي منفصل لأن `pnpm@10.18.3` لم يكن متاحًا في مخزونه الافتراضي. شُغّلت أوامر المشروع بعد توفير النسخة المحددة في `packageManager`.

## CI

- نجح CI في PR #8 على Windows وLinux وmacOS 15 Intel، إضافة إلى Docs وSecurity وPR body policy.
- استُخدم runner `macos-15-intel` بعد أن ألغى GitHub وظائف `macos-latest` و`macos-15` (arm64) وهي في الطابور، مع إبقاء اسمي الفحصين `Foundation (macos-latest)` و`Build & test (macos-latest)` للتوافق مع حماية الفرع. يحافظ ذلك على تغطية macOS لكنه لا يغطي arm64.
- نجحت الوظائف: Foundation على الأنظمة الثلاثة، وBuild & test على Windows وLinux وmacOS 15 Intel.

## ما لم يُختبر

- macOS arm64 (`macos-latest` أو `macos-15`) لم يُختبر في هذا PR بسبب إلغاء GitHub للوظائف في الطابور؛ غطّى CI صورة macOS 15 Intel.
- المراجعة البصرية البشرية للقطتين لم تُسجّل بعد؛ التحقق الآلي فقط هو ما تم.
- لم يُختبر المثبّت ولا الإصدارات النظيفة على Windows 10 وWindows 11.
- لا تنزيل أو طلب شبكة حقيقي ضمن هذه المرحلة.
