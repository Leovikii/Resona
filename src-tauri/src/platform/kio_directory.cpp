// SPDX-License-Identifier: GPL-3.0-only

#include <KIO/ListJob>
#include <QCoreApplication>
#include <QEventLoopLocker>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QPointer>
#include <QSocketNotifier>
#include <QTimer>
#include <QUrl>
#include <cerrno>
#include <cstdio>
#include <fcntl.h>
#include <unistd.h>

// Run only in a child process, before GTK, so Qt owns its event loop and can be terminated.
extern "C" int resona_kio_directory() {
    int argc = 1;
    char name[] = "resona-directory-worker";
    char *argv[] = {name, nullptr};
    QCoreApplication app(argc, argv);
    const QEventLoopLocker keepAlive; // KIO releases its quit lock after each job; stdin owns our lifetime.
    const int flags = fcntl(STDIN_FILENO, F_GETFL);
    if (flags == -1 || fcntl(STDIN_FILENO, F_SETFL, flags | O_NONBLOCK) == -1) return 2;
    QSocketNotifier input(STDIN_FILENO, QSocketNotifier::Read);
    QTimer deadline;
    deadline.setSingleShot(true);
    QByteArray request;
    QJsonArray names;
    QPointer<KIO::ListJob> pending;
    int failure = 0;
    QObject::connect(&deadline, &QTimer::timeout, &app, [&] { app.exit(7); });
    QObject::connect(&input, &QSocketNotifier::activated, &app, [&] {
        char bytes[4096];
        const auto count = read(STDIN_FILENO, bytes, sizeof(bytes));
        if (count == 0) { app.quit(); return; }
        if (count < 0) {
            if (errno != EAGAIN && errno != EINTR) app.exit(2);
            return;
        }
        request.append(bytes, count);
        if (request.size() > 65537) { app.exit(2); return; }
        if (!request.endsWith('\n')) return;
        // KIO-FUSE remoteUrl uses Qt's readable form, which can contain literal spaces/brackets.
        const QUrl url(QJsonDocument::fromJson(request).object().value(QStringLiteral("uri")).toString(),
            QUrl::TolerantMode);
        request.clear();
        if (!url.isValid() || url.scheme() != QStringLiteral("nfs") || url.host().isEmpty()
            || !url.userInfo().isEmpty()) { app.exit(2); return; }
        input.setEnabled(false);
        names = QJsonArray();
        // Some NFS workers report '.' as a regular file; FUSE then replaces the directory node.
        auto *job = KIO::listDir(url, KIO::HideProgressInfo,
            KIO::ListJob::ListFlag::IncludeHidden | KIO::ListJob::ListFlag::ExcludeDotAndDotDot);
        pending = job;
        QObject::connect(job, &KIO::ListJob::entries, &app,
            [&](KIO::Job *, const KIO::UDSEntryList &entries) {
                for (const auto &entry : entries) {
                    const auto name = entry.stringValue(KIO::UDSEntry::UDS_NAME);
                    if (name == QStringLiteral(".") || name == QStringLiteral("..")) continue;
                    if (name.isEmpty() || name.contains(QLatin1Char('/')) || name.contains(QChar(0))) {
                        failure = 3; app.exit(failure); return;
                    }
                    names.append(name);
                    if (names.size() > 100000) { failure = 4; app.exit(failure); return; }
                }
            });
        QObject::connect(job, &KJob::result, &app, [&](KJob *result) {
            deadline.stop();
            if (failure || result->error()) {
                std::fprintf(stderr, "KIO directory error: %d\n", result->error());
                app.exit(failure ? failure : result->error());
                return;
            }
            auto output = QJsonDocument(names).toJson(QJsonDocument::Compact);
            output.append('\n');
            if (output.size() > 16 * 1024 * 1024
                || std::fwrite(output.constData(), 1, output.size(), stdout) != size_t(output.size())
                || std::fflush(stdout)) { app.exit(6); return; }
            input.setEnabled(true);
        });
        deadline.start(15000);
    });
    const int result = app.exec();
    if (pending) pending->kill();
    return result;
}
