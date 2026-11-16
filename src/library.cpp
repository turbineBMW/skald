#include "library.h"

#include <QJsonObject>

static QStringList strings(const QJsonValue &value) {
    QStringList list;
    for (const auto &item : value.toArray())
        if (!item.toString().isEmpty())
            list << item.toString();
    return list;
}

QList<Book> Book::listFromJson(const QJsonArray &array) {
    QList<Book> books;
    for (const auto &value : array) {
        const QJsonObject o = value.toObject();
        Book book;
        book.asin = o.value("asin").toString();
        book.title = o.value("title").toString();
        if (book.asin.isEmpty() || book.title.isEmpty())
            continue;
        book.authors = strings(o.value("authors"));
        book.narrators = strings(o.value("narrators"));
        book.series = o.value("series").toString();
        book.cover = o.value("cover").toString();
        book.percent = qBound(0, o.value("percent").toInt(), 100);
        book.remaining = o.value("remaining").toString();
        book.finished = o.value("finished").toBool();
        books << book;
    }
    return books;
}

QJsonArray Book::listToJson(const QList<Book> &books) {
    QJsonArray array;
    for (const auto &book : books)
        array << QJsonObject{
            {"asin", book.asin}, {"title", book.title},
            {"authors", QJsonArray::fromStringList(book.authors)},
            {"narrators", QJsonArray::fromStringList(book.narrators)},
            {"series", book.series}, {"cover", book.cover}, {"percent", book.percent},
            {"remaining", book.remaining}, {"finished", book.finished}};
    return array;
}

// Every word has to appear somewhere in the title, people, or series.
bool Book::matches(const QStringList &words) const {
    const QString haystack = (title + ' ' + authors.join(' ') + ' ' + narrators.join(' ') + ' ' + series).toCaseFolded();
    for (const auto &word : words)
        if (!haystack.contains(word))
            return false;
    return true;
}

int LibraryModel::rowCount(const QModelIndex &parent) const {
    return parent.isValid() ? 0 : m_visible.size();
}

QVariant LibraryModel::data(const QModelIndex &index, int role) const {
    if (!index.isValid() || index.row() < 0 || index.row() >= m_visible.size())
        return {};
    const Book &book = m_books.at(m_visible.at(index.row()));
    switch (role) {
    case Asin: return book.asin;
    case Title: return book.title;
    case Authors: return book.authors.join(", ");
    case Narrators: return book.narrators.join(", ");
    case Series: return book.series;
    case Cover: return book.cover;
    case Percent: return book.percent;
    case Remaining: return book.remaining;
    case Finished: return book.finished;
    }
    return {};
}

QHash<int, QByteArray> LibraryModel::roleNames() const {
    return {{Asin, "asin"}, {Title, "title"}, {Authors, "authors"}, {Narrators, "narrators"}, {Series, "series"},
            {Cover, "cover"}, {Percent, "percent"}, {Remaining, "remaining"}, {Finished, "finished"}};
}

const Book *LibraryModel::find(const QString &asin) const {
    for (const auto &book : m_books)
        if (book.asin == asin)
            return &book;
    return nullptr;
}

QString LibraryModel::asinAt(int row) const {
    return row >= 0 && row < m_visible.size() ? m_books.at(m_visible.at(row)).asin : QString();
}

int LibraryModel::rowOf(const QString &asin) const {
    for (int row = 0; row < m_visible.size(); ++row)
        if (m_books.at(m_visible.at(row)).asin == asin)
            return row;
    return -1;
}

void LibraryModel::setBooks(const QList<Book> &books) {
    beginResetModel();
    m_books = books;
    rebuild();
    endResetModel();
    emit countChanged();
}

void LibraryModel::setFilter(const QString &filter) {
    if (filter == m_filter)
        return;
    beginResetModel();
    m_filter = filter;
    rebuild();
    endResetModel();
    emit countChanged();
}

void LibraryModel::rebuild() {
    const QStringList words = m_filter.toCaseFolded().split(' ', Qt::SkipEmptyParts);
    m_visible.clear();
    for (int i = 0; i < m_books.size(); ++i)
        if (words.isEmpty() || m_books.at(i).matches(words))
            m_visible << i;
}
