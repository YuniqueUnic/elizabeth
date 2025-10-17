# justfile settings
set dotenv-load


# justfile recipes
default:
    @just --list

current:
    #/Users/unic/dev/projs/rs/elizabeth
    @pwd

# update sqlx migrations and sqlx prepare
sqlx:
    @echo "sqlx migrate run"
    # TODO: get all migration files by order
    # detect the db file, if not exists, create it by sqlite3 with migration files
    # run sqlx migrate run
    # run cargo sqlx prepare --workspace


migration-files:
    @ls crates/board/migrations/*.sql | xargs -I {} echo "{}"
