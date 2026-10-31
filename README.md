# تنزيل — Tanzeel

> برنامج سطح مكتب لنظام ويندوز لتنزيل الملفات والفيديوهات — عربي أولًا، مع واجهة إنجليزية بضغطة زر.

**الحالة: قيد البناء — المرحلة 1 (الهيكل واللغة والإعدادات).**

أُنجزت المرحلة 0 (الوثائق والحوكمة) ودُمجت. يوجد الآن هيكل تطبيق أولي مبني على Tauri 2 وReact وTypeScript ونواة Rust وSQLite: يفتح التطبيق، ويعرض واجهة عربية RTL افتراضيًا، ويبدّل إلى الإنجليزية فورًا، ويحفظ الإعدادات محليًا. **لا يوجد تنزيل ولا اتصال شبكة بعد**، وكل ميزة تنزيل ما زالت **لم تُختبر** لأنها لم تُبنَ. التفاصيل في [docs/ROADMAP.md](docs/ROADMAP.md).

---

## الفكرة

**تنزيل** مدير تنزيل لسطح المكتب يجمع في مكان واحد:

- تنزيل الملفات المباشرة من روابط `HTTP/HTTPS`، مع **تسريع عبر تقسيم الملف إلى اتصالات متعددة** حين يسمح الخادم.
- تنزيل الفيديوهات من يوتيوب والمنصات الأخرى التي يدعمها محرك التنزيل.
- التقاط الروابط من المتصفح بضغطة من المستخدم، عبر إضافة متصفح وقناة رسائل أصلية آمنة.
- جدولة التنزيلات في وقت محدد.
- سحب محدود لروابط نوع معيّن من صفحة واحدة، بحدود صريحة واحترام `robots.txt`.
- اختيار الصيغة والجودة المتاحة من المصدر، لا جودة مفترضة.
- إدارة قائمة انتظار واضحة: إيقاف، استئناف، إلغاء، إعادة محاولة.
- سجل محلي للتنزيلات، وإعدادات محفوظة، وواجهة عربية كاملة من اليمين إلى اليسار.
- تبديل فوري إلى الإنجليزية دون إعادة تشغيل البرنامج.

> **ملاحظة نطاق:** أقرّ المالك في 2026-10-31 إدخال التسريع وتكامل المتصفح والجدولة وسحب الموقع في نطاق v1.0. هذا يوسّع نطاق الإصدار الأول، وكل ميزة منها موسومة **لم تُختبر**، ولها ورقة قرار مستقلة.

الهدف ليس نسخ برنامج آخر، بل بناء بديل وظيفي مستقل، نظيف الترخيص، واضح الحدود، ويحترم القانون وشروط استخدام المنصات.

## علاقة المشروع ببرنامج «زاجل»

المشروع بديل وظيفي لبرنامج «زاجل» الموجود على جهاز المالك، لكنه **مستقل تمامًا**:

- وصف المالك أن «زاجل» يشبه **IDM** في وظيفته، أي مدير تنزيل يسرّع التنزيل ويستأنفه ويتكامل مع المتصفح. هذه معلومة من المالك، وليست نتيجة فحص للبرنامج.
- لا نستخدم كود «زاجل» ولا تصميمه ولا اسمه التجاري ولا ملفاته، ولا كود IDM أو تصميمه أو اسمه.
- لا نفكّ ملفات أي من البرنامجين ولا نستخرج منها شيئًا.
- لا نعتمد إلا على المعلومات المنشورة علنًا، وما يذكره المالك عن وظيفته العامة.
- لم نعثر في البحث العام على معلومات منشورة مؤكدة عن برنامج «زاجل» كأداة تنزيل، لذلك لا تنسب هذه الوثائق إليه أي تفصيل تقني.
- أي تشابه في الوظيفة العامة مقصود لأنهما يؤدّيان غرضًا مشتركًا، وليس نسخًا.

## ما الذي سيفعله البرنامج

| المجال | الوصف المختصر | الحالة |
| --- | --- | --- |
| تنزيل الملفات | روابط مباشرة `HTTP/HTTPS` مع استئناف وإعادة محاولة | مخطط |
| تسريع التنزيل | تقسيم الملف إلى أجزاء وتنزيلها بالتوازي ضمن حدود الخادم | مخطط — لم يُختبر |
| تكامل المتصفح | إضافة متصفح + قناة رسائل أصلية، التقاط بضغطة المستخدم | مخطط — لم يُختبر |
| الجدولة | تنزيل في وقت محدد مع بقاء التطبيق يعمل | مخطط — لم يُختبر |
| سحب الموقع | روابط نوع معيّن من صفحة واحدة بحدود واحترام `robots.txt` | مخطط — لم يُختبر |
| فيديوهات يوتيوب | جلب البيانات، عرض الصيغ، تنزيل الفيديو أو الصوت | مخطط |
| منصات أخرى | ما يدعمه محرك التنزيل ويكون مختبرًا ومصرحًا به | مخطط |
| قائمة انتظار | تقدم، أولوية، عدد تنزيلات متزامنة، إيقاف واستئناف | مخطط |
| الجودة | كل الصيغ والجودات التي يعلنها المصدر | مخطط |
| الصوت والترجمة | استخراج الصوت، دمج الصوت والفيديو، الترجمة والصور المصغرة | مخطط |
| الواجهة | عربية RTL افتراضيًا، إنجليزية بضغطة زر | مخطط |
| السجل | سجل محلي قابل للبحث والتصدير والحذف | مخطط |
| التحديثات | تحديثات موقعة من GitHub Releases فقط | مخطط |

## ما الذي لن يفعله البرنامج

- لن يتجاوز إدارة الحقوق الرقمية (DRM) أو الحماية التقنية.
- لن يفتح محتوى مدفوعًا أو خاصًا أو مقيدًا بعمر دون حق مشروع.
- لن يتجاوز حظرًا جغرافيًا أو شروط استخدام منصة.
- لن يلتقط تنزيلات من المتصفح تلقائيًا أو خفيًا؛ الالتقاط بضغطة من المستخدم وقابل للتعطيل.
- لن يفتح منفذًا محليًا (localhost) لخدمة المتصفح في v1؛ الاتصال عبر قناة رسائل أصلية.
- لن يزحف خارج نطاق الصفحة التي يحددها المستخدم، ولن يتجاوز `robots.txt` أو حدود المعدل.
- لن يضمن دعم «كل المواقع» أو «كل الجودات»؛ يدعم ما يثبته الاختبار.
- لن يضمن سرعة معينة؛ التسريع يعتمد على دعم الخادم وحدوده، ويُقاس ويُعلَن ولا يُوعَد به.
- لن يضمّن محتوى محميًا بحقوق نشر.
- لن يرسل بيانات استخدام إلى خادم تابع للمشروع في الإصدار الأول.
- لن يعمل كخدمة سحابية أو حساب مستخدم أو مزامنة بين الأجهزة.
- لن يبني إصدارات يدويًا؛ الإصدارات من GitHub Actions فقط.

## التقنية المقترحة

هذه اختيارات مقترحة في وثائق القرار، وليست نهائية حتى موافقة المالك:

| الطبقة | الاقتراح | البدائل المدروسة |
| --- | --- | --- |
| الإطار | Tauri 2 | Electron، .NET/WPF، Flutter |
| لغة النواة | Rust | TypeScript فقط، C#، Go، Dart |
| الواجهة | React + TypeScript | Svelte، Vue، واجهة أصلية |
| التخزين | SQLite مع ترحيلات | ملفات JSON، Registry، IndexedDB |
| محرك الفيديو | yt-dlp كعملية فرعية مُدارَة | مستخرجات أصلية، Python مضمّن، أتمتة متصفح |
| الملفات المباشرة | محمّل HTTP أصلي في النواة مع تسريع اختياري | yt-dlp لكل شيء، مكتبة خارجية، aria2 |
| تكامل المتصفح | إضافة WebExtension + Native Messaging | منفذ localhost، مراقبة الحافظة فقط، بروتوكول مخصص |
| الجدولة | جدولة داخل التطبيق محفوظة في SQLite | Task Scheduler، خدمة ويندوز |
| سحب الموقع | زاحف HTML محدود في النواة | متصفح بلا واجهة، تعبيرات نمطية فقط |
| الترجمة | i18next مع RTL | حل مخصص، react-intl |
| الإصدارات | GitHub Actions + مثبّت موقّع + تحديث موقّع | بناء يدوي، Microsoft Store |

التفاصيل والأسباب في [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) و[docs/adr/](docs/adr/).

## التطوير والاختبار

المتطلبات: Node.js 22 أو أحدث، وpnpm 10.18.3 عبر Corepack، وRust stable مع Visual Studio Build Tools وWebView2 (ويندوز). لا يحتاج تشغيل الاختبارات إلى شبكة.

```bash
corepack pnpm install --frozen-lockfile
corepack pnpm lint
corepack pnpm typecheck
corepack pnpm test
corepack pnpm test:i18n
corepack pnpm build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --workspace
```

لتشغيل نافذة Tauri أثناء التطوير: `corepack pnpm tauri dev` (يتطلب Rust وWebView2). المثبّت خارج نطاق المرحلة 1.

## الوثائق

| الملف | المحتوى |
| --- | --- |
| [docs/PRD.md](docs/PRD.md) | المتطلبات، المستخدمون، النطاق، وما لا يفعله المشروع |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | الأجزاء، حدودها، وكيف يتصل بعضها ببعض |
| [docs/ROADMAP.md](docs/ROADMAP.md) | المراحل المرقّمة وشروط النجاح وما خارج كل مرحلة |
| [docs/adr/](docs/adr/) | قرارات كبيرة: الرخصة، الإطار، اللغة، التخزين، المحرك، الترجمة، الإصدارات، الأمان، التسريع، المتصفح، الجدولة، سحب الموقع |
| [AGENTS.md](AGENTS.md) | قواعد العمل الإلزامية لأي وكيل أو مساهم |
| [.github/workflows/ci.yml](.github/workflows/ci.yml) | البناء والاختبار التلقائي على ويندوز ولينكس وماك |
| [.github/workflows/pr-policy.yml](.github/workflows/pr-policy.yml) | التحقق من رسالة PR ووجود البرومبت والاختبارات |

## الرخصة

**GNU General Public License v3.0 أو أحدث (`GPL-3.0-or-later`).**

السبب المختصر: المشروع يوزّع أدوات وسائط قد تكون مرخّصة بـ GPL، والرخصة تمنع إغلاق النسخ المعدّلة وتوفّر حماية براءات اختراع. اعتمد المالك هذا الخيار في 2026-10-30، انظر [ADR-0001](docs/adr/0001-license.md). هذا ليس رأيًا قانونيًا.

## حقوق النشر

Copyright (C) 2026 Tanzeel contributors.

## المساهمة

كل مرحلة = Issue. كل تغيير = فرع صغير وPull Request واحد يفعل شيئًا واحدًا. لا دفع مباشر إلى `main`. لا دمج قبل نجاح الاختبارات على كل الأنظمة والمراجعة. التفاصيل الملزمة في [AGENTS.md](AGENTS.md).

## إخلاء مسؤولية

«تنزيل» أداة محايدة. المستخدم مسؤول عن امتلاك الحق في تنزيل المحتوى وعن الالتزام بقوانين بلده وشروط استخدام المنصة. المشروع غير تابع لـ «زاجل» أو يوتيوب أو أي منصة أخرى، ولا يستخدم علاماتها التجارية.

---

## English (brief)

**Tanzeel** is a Windows desktop download manager for direct files and videos from YouTube and other supported platforms. It is Arabic-first with an instant English toggle. Planned v1.0 scope includes segmented (multi-connection) acceleration, opt-in browser capture through a WebExtension and native messaging, scheduled downloads, and a bounded single-page link grabber. This repository now contains the foundation documents plus a Phase 1 scaffold (Tauri 2, React/TypeScript, Rust core, SQLite settings) that opens an Arabic-first interface with an instant English toggle. **No download feature exists or has been tested yet**, and no network request is made.

The project is an independent functional alternative to a locally installed program named “Zagel”, which the owner describes as similar in purpose to IDM (a download manager). It does not use, inspect, copy, or reverse-engineer that program or IDM. Only publicly published information, plus the owner’s general description, may be referenced.

Proposed stack: Tauri 2 + Rust core + React/TypeScript UI + SQLite + managed `yt-dlp`/`ffmpeg` sidecars. The desktop framework, core language, storage, i18n, and testing choices are approved for Phase 1; the download engine and release choices remain proposals awaiting owner approval. License: GPL-3.0-or-later (approved).

Read [AGENTS.md](AGENTS.md) before contributing. No direct pushes to `main`; every change needs a small PR with tests and the prompt that produced it.
