CREATE TABLE IF NOT EXISTS {{ CHANNEL_NAME }}_MARKOV {
    word TEXT,
    pred TEXT,
    succ TEXT,
}