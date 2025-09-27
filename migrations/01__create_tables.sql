CREATE TABLE plushies
(
    id    SERIAL PRIMARY KEY,
    name  TEXT    NOT NULL,
    price DOUBLE PRECISION NOT NULL
);

CREATE TABLE customer
(
    id       SERIAL PRIMARY KEY,
    name     TEXT NOT NULL,
    password TEXT NOT NULL
);

CREATE TABLE orders
(
    id           SERIAL PRIMARY KEY,
    order_date   TIMESTAMP      NOT NULL DEFAULT CURRENT_TIMESTAMP,
    total_amount DOUBLE PRECISION NOT NULL,
    customer_id  INTEGER        NOT NULL REFERENCES customer (id)
);

CREATE TABLE order_plush (
                             order_id  INTEGER NOT NULL REFERENCES orders(id),
                             plush_id  INTEGER NOT NULL REFERENCES plushies(id),
                             PRIMARY KEY(order_id, plush_id)
);