CREATE TABLE FileEntries (
    id UUID PRIMARY KEY,

    filename TEXT,
    content_type TEXT,

    download_count_type: DownloadCount,
    download_count: u64,

    visibility: Visibility,
    password: Password,

    lifetime: Lifetime,
    upload_date: SystemTime,

);

