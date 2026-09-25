-- SQL schema for Yukthi File Storage (YFS)

-- Users
CREATE TABLE users (
    user_id UUID PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    domain VARCHAR(255) NOT NULL,

    organization_id UUID NOT NULL,
    organization_name VARCHAR(250) NOT NULL,

    private_info JSONB NOT NULL,    -- Available only to the user themselves
    public_info JSONB NOT NULL,     -- Available to all within the organization

    last_seen_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL
);


-- User Session management table
CREATE TABLE sessions (
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,

    refresh_token UUID PRIMARY KEY,

    sso_token VARCHAR(50) NOT NULL,
    fcm_token TEXT NULL,

    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL DEFAULT (CURRENT_TIMESTAMP + INTERVAL '3 days')
);


-- Folders
CREATE TABLE folders (
    folder_id UUID PRIMARY KEY,
    parent_folder_id UUID NULL REFERENCES folders(folder_id) ON DELETE CASCADE,

    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,

    folder_name VARCHAR(255) NOT NULL,
    folder_info JSONB NOT NULL, -- For UI related information like color, icon, etc

    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    deleted_at TIMESTAMPTZ NULL, -- If this is not null, it means the folder is deleted, show in trash view

    UNIQUE NULLS NOT DISTINCT (user_id, parent_folder_id, folder_name) -- Ensure unique folder names within the same parent folder and user
);


-- Files
CREATE TABLE files (
    file_id UUID PRIMARY KEY,
    folder_id UUID NOT NULL REFERENCES folders(folder_id) ON DELETE CASCADE,

    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,

    file_name VARCHAR(255) NOT NULL,
    file_info JSONB NOT NULL, -- For UI related information like color, icon, etc
    is_locked BOOLEAN NOT NULL DEFAULT TRUE,

    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    deleted_at TIMESTAMPTZ NULL, -- If this is not null, it means the file is deleted, show in trash view

    UNIQUE (folder_id, file_name) -- Ensure unique file names within the same folder
);


-- File Versions (To keep track of all versions of a file)
CREATE TABLE file_versions (
    file_id UUID NOT NULL REFERENCES files(file_id) ON DELETE CASCADE,
    file_version INTEGER NOT NULL,

    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    hosted_at VARCHAR(255) NOT NULL REFERENCES servers(host_address) ON DELETE CASCADE,

    file_location TEXT NOT NULL, -- Location of the file in the storage system (IP:port/org-id/user-id/file-id[:2]/file-id)
    file_size BIGINT NOT NULL, -- Size of the file in bytes

    metadata JSONB NOT NULL, -- For storing file metadata like type, tags, etc
    file_hash TEXT NOT NULL, -- Hash of the file for integrity verification
    -- TODO: Index on file_hash as required for the duplicate detection

    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,

    PRIMARY KEY (file_id, file_version) -- Ensure unique file versions for each file
);


-- Internal sharing (Only folders can be shared internally and only to users within the same organization)
CREATE TABLE internal_shares (
    folder_id UUID NOT NULL REFERENCES folders(folder_id) ON DELETE CASCADE,

    created_by UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    shared_with_user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,

    can_preview BOOLEAN NOT NULL DEFAULT FALSE,
    can_download BOOLEAN NOT NULL DEFAULT FALSE,
    can_create BOOLEAN NOT NULL DEFAULT FALSE,
    can_update BOOLEAN NOT NULL DEFAULT FALSE,
    can_delete BOOLEAN NOT NULL DEFAULT FALSE,

    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PRIMARY KEY (folder_id, shared_with_user_id) -- Ensure unique internal shares for each folder and user that it is shared with
);


-- External sharing (Both files and folders can be shared externally, but only to users outside the organization)
CREATE TABLE external_shares (
    share_id VARCHAR(36) PRIMARY KEY,  -- Unique identifier for the share (will be used in the shareable link)

    created_by UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,

    share_file_target_id UUID NULL REFERENCES files(file_id) ON DELETE CASCADE,
    share_folder_target_id UUID NULL REFERENCES folders(folder_id) ON DELETE CASCADE,

    can_preview BOOLEAN NOT NULL DEFAULT FALSE,
    can_download BOOLEAN NOT NULL DEFAULT FALSE,
    can_create BOOLEAN NOT NULL DEFAULT FALSE,
    can_update BOOLEAN NOT NULL DEFAULT FALSE,
    can_delete BOOLEAN NOT NULL DEFAULT FALSE,

    share_info JSONB NOT NULL, -- For UI related information like share notes, etc
    password_hash TEXT NULL, -- Optional password for accessing the shared resource
    phones_for_otp TEXT[] NOT NULL DEFAULT '{}', -- Optional phone number for accessing the shared resource (if provided, an OTP will be sent to this number for verification)
    emails_for_otp TEXT[] NOT NULL DEFAULT '{}', -- Optional email for accessing the shared resource (if provided, an OTP will be sent to this email for verification)
    expires_at TIMESTAMPTZ NULL, -- Optional expiration date for the share, cron job will delete the share after this date

    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,

    CHECK ( -- This is an XOR (exactly one must be non-null)
        (share_file_target_id IS NOT NULL) <>
        (share_folder_target_id IS NOT NULL)
    ); -- Ensure that either a file or a folder is shared, but not both
);


-- User Quota
CREATE TABLE user_quotas (
    user_id UUID PRIMARY KEY REFERENCES users(user_id) ON DELETE CASCADE,

    used_storage_bytes BIGINT NOT NULL DEFAULT 0, -- Storage currently used by the user
    used_file_count INT NOT NULL DEFAULT 0 -- Number of files currently used by the user
);


-- Servers
CREATE TABLE servers (
    host_address VARCHAR(255) PRIMARY KEY,
    -- TODO: Use the API key and servers in the code and not by Environment Variables
    secret_key TEXT NOT NULL,   -- API Key for authenticating requests to this server

    -- Optional reference to the Organization ID
    -- If present then this server is dedicated to that organization
    -- else it is a general-purpose server
    dedicated_to_organization_id UUID NULL,

    server_name VARCHAR(255) NOT NULL,
    server_description TEXT NOT NULL,

    quota_allocated_bytes BIGINT NOT NULL,
    quota_utilized_bytes BIGINT NOT NULL
);


-- Indexes for performance optimization
CREATE INDEX idx_files_folder_id ON files(folder_id);
CREATE INDEX idx_internal_shares_created_by ON internal_shares(created_by);
CREATE INDEX idx_internal_shares_shared_with_user_id ON internal_shares(shared_with_user_id);
CREATE INDEX idx_external_shares_created_by ON external_shares(created_by);
CREATE INDEX idx_external_shares_share_file_target_id ON external_shares(share_file_target_id);
CREATE INDEX idx_external_shares_share_folder_target_id ON external_shares(share_folder_target_id);
CREATE INDEX idx_folders_user_id ON folders(user_id);
CREATE INDEX idx_file_versions_file ON file_versions(file_id);
CREATE INDEX idx_external_expiry ON external_shares(expires_at);
CREATE INDEX idx_deleted_files ON files(deleted_at);
CREATE INDEX idx_deleted_folders ON folders(deleted_at);
CREATE INDEX idx_files_expired_locks ON files (updated_at) WHERE is_locked = TRUE;
CREATE INDEX idx_files_orphan_cleanup ON files (created_at, file_id);
CREATE INDEX CONCURRENTLY idx_folders_parent_folder_id ON folders(parent_folder_id);
