# AI Rust workflow creator

Playing around with a system similar to how Zapier does it. Very much incomplete, but I learned a lot from it.

## Learnings

- I could not find a _native_ way to handle `.env` files in Rust.

  - Yes, I can always write my own parser, but it would be nice if it worked in similar fashion Node.js does it.

    ```shell
    node --env-file=.env app.js
    ```

- **Docker volumes** are files that persist on the host machine. They, by default, will not be deleted when container is deleted.

  - This could be very useful for any kind of database data.

  - In our case, **we are using the `volumes` to load `init.sql` file into the container when it starts**.

    - See [_"Initialization scripts"_](https://hub.docker.com/_/postgres/).

- **For efficient searching based on the embedding column, consider using the `HNSW` index on that column**.

  - You can read more about `pgvector` embedding indexing [here](https://github.com/pgvector/pgvector?tab=readme-ov-file#indexing).

- When creating an index, you can either specify the index name, or omit it.

  ```sql
  <!-- You could omit the `my_name` here! -->
  create index if not exists my_name on actions(id) where ...
  ```

- You can configure custom `cargo` flags via `.cargo/config.toml` file.

  - When developing, I find the rules to disable the "dead code" and "unused variable" warnings quite useful.

- PostgreSQL has this nifty feature where you can **return data upon creating it**

  ```sql
  insert into apps (name, description) values ($1, $2) returning id
  ```

  This is very useful when the `id` is auto-generated!

- Rust is amazing, but I'm sometimes tripped up by the language semantics.

  - For example **I could not extract the `id` from the row, because I did not import the `Row` trait**.

    - Since there is no autocomplete in my IDE when I do not have trait imported, how am I supposed to know that the trait exists?

- There is a **difference between using `tokio::join` and `tokio::spawn`**.

  - The `tokio::join` will run the tasks concurrently in the same "task".

    - Much easier to work with than the `tokio::spawn` since you do not have to move anything between threads. There is no synchronization mechanism needed.

  - The `tokio::spawn` will run the tasks concurrently **on a single-thread runtime**. On a **multi-thread runtime**, the tasks might be run on separate threads.

    - While you **get the benefit of potential parallelism here**, it is **much harder to work with given the requirements of working with threads**.

      - You have to have the variables be annotated with lifetimes, or clone them.

- **Passing data across the "thread" boundary is quite cumbersome** and will require you to use `Arc` and `.copy` API quite a lot (most likely).

- We can even match on tuples in Rust. Pretty neat!

  ```rust
    match (args.command, args.description) {
        (Some(_), None) => {}
        (None, Some(_)) => {}
        (Some(_), Some(_)) => {}
        (None, None) => {}
    }
  ```

- **Not specific to PostgreSQL**, but you can **cast a value to another type when performing DB operations**.

  ```sql
  select name, (embedding <=> $1::vector) as cos_distance
  from actions
  where cos_distance > 0.4
  ```

  The `::vector` here is a cast of the input variable to the vector type. [Reference for `pgvector`](https://github.com/pgvector/pgvector/blob/049972a4a3a04e0f49de73d78915706377035f48/sql/vector.sql#L154).
