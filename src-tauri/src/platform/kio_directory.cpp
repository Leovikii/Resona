// SPDX-License-Identifier: GPL-3.0-only

#include <KIO/ListJob>
#include <QCoreApplication>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QPointer>
#include <QTimer>
#include <QUrl>
#include <cstdio>

// Run only in a child process, before GTK, so Qt owns its event loop and can be terminated.
extern "C" int resona_kio_directory() {
    int argc = 1;
    char name[] = "resona-directory-worker";
    char *argv[] = {name, nullptr};
    QCoreApplication app(argc, argv);
    QFile input;
    if (!input.open(stdin, QIODevice::ReadOnly)) return 2;
    const auto request = QJsonDocument::fromJson(input.read(65537)).object();
    const QUrl url(request.value(QStringLiteral("uri")).toString(), QUrl::StrictMode);
    if (!url.isValid() || url.scheme() != QStringLiteral("nfs") || url.host().isEmpty()
        || !url.userInfo().isEmpty()) return 2;

    QJsonArray names;
    // Some NFS workers report '.' as a regular file; FUSE then replaces the directory node.
    auto *job = KIO::listDir(url, KIO::HideProgressInfo,
        KIO::ListJob::ListFlag::IncludeHidden | KIO::ListJob::ListFlag::ExcludeDotAndDotDot);
    QPointer<KIO::ListJob> pending(job);
    QObject::connect(job, &KIO::ListJob::entries, &app,
        [&](KIO::Job *, const KIO::UDSEntryList &entries) {
            for (const auto &entry : entries) {
                const auto name = entry.stringValue(KIO::UDSEntry::UDS_NAME);
                if (name == QStringLiteral(".") || name == QStringLiteral("..")) continue;
                if (name.isEmpty() || name.contains(QLatin1Char('/')) || name.contains(QChar(0))) {
                    app.exit(3);
                    return;
                }
                names.append(name);
                if (names.size() > 100000) {
                    app.exit(4);
                    return;
                }
            }
        });
    QObject::connect(job, &KJob::result, &app, [&](KJob *result) {
        if (result->error()) {
            std::fprintf(stderr, "KIO directory error: %d\n", result->error());
            app.exit(result->error());
            return;
        }
        const auto output = QJsonDocument(names).toJson(QJsonDocument::Compact);
        if (std::fwrite(output.constData(), 1, output.size(), stdout) != size_t(output.size())) {
            app.exit(6);
            return;
        }
        app.quit();
    });
    QTimer::singleShot(15000, &app, [&] { app.exit(7); });
    const int result = app.exec();
    if (pending) pending->kill();
    return result;
}
