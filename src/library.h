#pragma once

#include <QAbstractListModel>
#include <QJsonArray>
#include <QList>
#include <QString>
#include <QStringList>

struct Book {
    QString asin;
    QString title;
    QStringList authors;
    QStringList narrators;
    QString series;
    QString cover;
    int percent = 0;
    QString remaining;
    bool finished = false;

    static QList<Book> listFromJson(const QJsonArray &array);
    static QJsonArray listToJson(const QList<Book> &books);
    bool matches(const QStringList &words) const;
};

// The user's titles, in the order Audible lists them, narrowed by a search.
class LibraryModel : public QAbstractListModel {
    Q_OBJECT
    Q_PROPERTY(int count READ count NOTIFY countChanged)
    Q_PROPERTY(int total READ total NOTIFY countChanged)

public:
    enum Role { Asin = Qt::UserRole + 1, Title, Authors, Narrators, Series, Cover, Percent, Remaining, Finished };

    explicit LibraryModel(QObject *parent = nullptr) : QAbstractListModel(parent) {}

    int rowCount(const QModelIndex &parent = {}) const override;
    QVariant data(const QModelIndex &index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;

    int count() const { return m_visible.size(); }
    int total() const { return m_books.size(); }
    const QList<Book> &books() const { return m_books; }
    const Book *find(const QString &asin) const;
    Q_INVOKABLE QString asinAt(int row) const;
    Q_INVOKABLE int rowOf(const QString &asin) const;

    void setBooks(const QList<Book> &books);
    void setFilter(const QString &filter);

signals:
    void countChanged();

private:
    void rebuild();

    QList<Book> m_books;
    QList<int> m_visible;
    QString m_filter;
};
