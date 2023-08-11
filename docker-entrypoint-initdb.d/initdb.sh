set -e
psql -U admin sampledb <<EOSQL
CREATE TABLE bookshelf_user (
  id text NOT NULL PRIMARY KEY,
  created_at timestamp NOT NULL default current_timestamp,
  updated_at timestamp NOT NULL default current_timestamp
);
EOSQL
