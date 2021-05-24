set -e
psql -U admin sampledb <<EOSQL
CREATE TABLE Accounts (
  account_id        SERIAL PRIMARY KEY,
  account_name      VARCHAR(20),
  first_name        VARCHAR(20),
  last_name         VARCHAR(20),
  email             VARCHAR(100),
  password_hash     CHAR(64),
  portrait_image    BYTEA,
  hourly_rate       NUMERIC(9,2)
);

CREATE TABLE BugStatus (
  status            VARCHAR(20) PRIMARY KEY
);

CREATE TABLE Bugs (
  bug_id            SERIAL PRIMARY KEY,
  date_reported     DATE NOT NULL,
  summary           VARCHAR(80),
  description       VARCHAR(1000),
  resolution        VARCHAR(1000),
  reported_by       BIGINT NOT NULL,
  assigned_to       BIGINT,
  verified_by       BIGINT,
  status            VARCHAR(20) NOT NULL DEFAULT 'NEW',
  priority          VARCHAR(20),
  hours             NUMERIC(9,2),
  FOREIGN KEY (reported_by) REFERENCES Accounts(account_id),
  FOREIGN KEY (assigned_to) REFERENCES Accounts(account_id),
  FOREIGN KEY (verified_by) REFERENCES Accounts(account_id),
  FOREIGN KEY (status) REFERENCES BugStatus(status)
);

CREATE TABLE Comments (
  comment_id        SERIAL PRIMARY KEY,
  bug_id            BIGINT NOT NULL,
  -- Adjacency List
  -- parent_id         BIGINT,
  -- Path Enumeration
  -- path              VARCHAR(1000),
  -- Nested Set
  -- nsleft            INTEGER NOT NULL,
  -- nsright           INTEGER NOT NULL,
  author            BIGINT NOT NULL,
  comment_date      TIMESTAMP WITH TIME ZONE NOT NULL,
  comment           TEXT NOT NULL,
  -- Adjacency List
  -- FOREIGN KEY (parent_id) REFERENCES Comments(comment_id),
  FOREIGN KEY (bug_id) REFERENCES Bugs(bug_id),
  FOREIGN KEY (author) REFERENCES Accounts(account_id)
);

CREATE TABLE CommentTree (
  ancestor          BIGINT NOT NULL,
  descendant        BIGINT NOT NULL,
  PRIMARY KEY (ancestor, descendant),
  FOREIGN KEY (ancestor) REFERENCES Comments(comment_id),
  FOREIGN KEY (descendant) REFERENCES Comments(comment_id)
);

CREATE TABLE Screenshots (
  bug_id            BIGINT NOT NULL,
  image_id          BIGINT NOT NULL,
  screenshot_image  BYTEA,
  caption           VARCHAR(100),
  PRIMARY KEY (bug_id, image_id),
  FOREIGN KEY (bug_id) REFERENCES Bugs(bug_id)
);

CREATE TABLE Tags (
  bug_id            BIGINT NOT NULL,
  tag               VARCHAR(20) NOT NULL,
  PRIMARY KEY (bug_id, tag),
  FOREIGN KEY (bug_id) REFERENCES Bugs(bug_id)
);

CREATE TABLE Products (
  product_id        SERIAL PRIMARY KEY,
  product_name      VARCHAR(50)
);

CREATE TABLE BugProducts (
  bug_id            BIGINT NOT NULL,
  product_id        BIGINT NOT NULL,
  PRIMARY KEY (bug_id, product_id),
  FOREIGN KEY (bug_id) REFERENCES Bugs(bug_id),
  FOREIGN KEY (product_id) REFERENCES Products(product_id)
);

-- data

INSERT INTO BugStatus (status) VALUES ('NEW');

INSERT INTO Accounts (account_name)
VALUES ('Fran'), ('Ollie'), ('Kukla');

INSERT INTO Bugs (date_reported, summary, reported_by)
VALUES (date(now()), 'The Bug', 1);

/* Adjacency List
INSERT INTO Comments (bug_id, parent_id, author, comment_date, comment)
VALUES (1, NULL, 1, now(), 'このバグの原因は何かな?'),
       (1, 1,    2, now(), 'ヌルポインターのせいじゃないかな?'),
       (1, 2,    1, now(), 'そうじゃないよ。それは確認済なんだ。'),
       (1, 1,    3, now(), '無効な入力を調べてみたら?'),
       (1, 4,    2, now(), 'そうか、バグの原因はそれだな。'),
       (1, 4,    1, now(), 'よし、じゃあチェック機能を追加してもらえるかな?'),
       (1, 6,    3, now(), '了解。修正したよ。');

WITH RECURSIVE CommentTree (comment_id, bug_id, parent_id, author, comment_date, comment, depth)
AS (
  -- 基底
  SELECT c.*, 0 AS depth FROM Comments c WHERE c.parent_id IS NULL
  UNION ALL
  -- 帰納
  SELECT c.*, ct.depth + 1 AS depth FROM CommentTree ct
  JOIN Comments c ON ct.comment_id = c.parent_id
)
SELECT * FROM CommentTree WHERE bug_id = 1;
*/

/* Path Enumeration
INSERT INTO Comments (bug_id, path, author, comment_date, comment)
VALUES (1, '1/',       1, now(), 'このバグの原因は何かな?'),
       (1, '1/2/',     2, now(), 'ヌルポインターのせいじゃないかな?'),
       (1, '1/2/3/',   1, now(), 'そうじゃないよ。それは確認済なんだ。'),
       (1, '1/4/',     3, now(), '無効な入力を調べてみたら?'),
       (1, '1/4/5/',   2, now(), 'そうか、バグの原因はそれだな。'),
       (1, '1/4/6/',   1, now(), 'よし、じゃあチェック機能を追加してもらえるかな?'),
       (1, '1/4/6/7/', 3, now(), '了解。修正したよ。');

SELECT * FROM Comments WHERE '1/4/6/7/' LIKE path || '%';
SELECT * FROM Comments WHERE path LIKE '1/4/%';
*/

/* Nested Set
INSERT INTO Comments (bug_id, nsleft, nsright, author, comment_date, comment)
VALUES (1,  1, 14,    1, now(), 'このバグの原因は何かな?'),
       (1,  2,  5,    2, now(), 'ヌルポインターのせいじゃないかな?'),
       (1,  3,  4,    1, now(), 'そうじゃないよ。それは確認済なんだ。'),
       (1,  6, 13,    3, now(), '無効な入力を調べてみたら?'),
       (1,  7,  8,    2, now(), 'そうか、バグの原因はそれだな。'),
       (1,  9, 12,    1, now(), 'よし、じゃあチェック機能を追加してもらえるかな?'),
       (1, 10, 11,    3, now(), '了解。修正したよ。');

SELECT c2.*
FROM Comments c1
JOIN Comments c2 ON c2.nsleft BETWEEN c1.nsleft AND c1.nsright
WHERE c1.comment_id = 4;

SELECT c2.*
FROM Comments c1
JOIN Comments c2 ON c1.nsleft BETWEEN c2.nsleft AND c2.nsright
WHERE c1.comment_id = 6;
*/

INSERT INTO Comments (bug_id, author, comment_date, comment)
VALUES (1, 1, now(), 'このバグの原因は何かな?'),
       (1, 2, now(), 'ヌルポインターのせいじゃないかな?'),
       (1, 1, now(), 'そうじゃないよ。それは確認済なんだ。'),
       (1, 3, now(), '無効な入力を調べてみたら?'),
       (1, 2, now(), 'そうか、バグの原因はそれだな。'),
       (1, 1, now(), 'よし、じゃあチェック機能を追加してもらえるかな?'),
       (1, 3, now(), '了解。修正したよ。');

INSERT INTO CommentTree (ancestor, descendant)
VALUES (1,1), (1,2), (1,3), (1,4), (1,5), (1,6), (1,7),
       (2,2), (2,3),
       (3,3),
       (4,4), (4,5), (4,6), (4,7),
       (5,5),
       (6,6), (6,7),
       (7,7);

SELECT c.*
FROM Comments c
JOIN CommentTree t ON c.comment_id = t.descendant
WHERE t.ancestor = 4;

SELECT c.*
FROM Comments c
JOIN CommentTree t ON c.comment_id = t.ancestor
WHERE t.descendant = 6;

EOSQL
