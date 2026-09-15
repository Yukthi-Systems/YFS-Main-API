# Features

- Search and filter files based on various criteria (e.g., name, type, size, date).
- Audit Logs
- User Management
- File sharing
- File versioning
- Server Management


---

## First release

- Done - Login is via SSO only
- Create a Office Files
- Done - Create a Folder or Sub-Folder
- Preview all Office suite files (if required do the video/audio streaming) [In the same GO Lang Files API]
- Upload any file (of any size - GO Lang : Tus)
- Play/Pause/Resume/Stop/Cancel/Internet Down - Should not break the Download or Upload files/folders (Tus)
- Self Editing of Office files (any)
- Download the files or preview the files or open in editor (if office)
- Delete a file or folder and all under that (Move to trash) and Permanently delete too
- List the files or folders - 2 Types of listing : Normal Personal Files and Folders ; Shared Folders view (list all the folders)
- Done - Share folder only no file sharing is allowed (Share externally as a link : Generate a link endpoint or Share internally to specific users : Link folder to users endpoint)
- Shared Folders have permissions : Preview, Download, Create/Upload, Edit/Update, Delete (Per user in the share) [Table View]
- External share have permissions again (same as internal share) but it will also have passwords / OTP - SMS|E-Mail (No login required, public pages)
- Versioning the files
- Mount the files to Windows / Mac / Linux - WebDav (or) Agent Based


---

## Endpoints for GO Lang Files API

- Internal : API-Key : Generate a token to upload / download a file (along with some metadata)
- Internal : API-Key : Cancel the Token + Delete the file if its upload
- Internal : API-Key : Delete the file
- Upload : Gen. Token : Play/Pause/Stop/Kill/Fail/etc . . . (After upload make a callback)
- Download : Gen. Token : Stream/View/Download/Buffer-View/etc . . . (No need to do any callbacks)
- Health Check : Internal for Docker (API-Key)

---

// This will generate the signed URL for the files listed below
[
    {
        "file_name": "example.txt",
        "file_type": "text/plain",
        "file_version": 1,
        "folder_id": "uuid",
        "expected_size": 1024
    },
    {
        "file_name": "example-2.txt",
        "file_type": "text/plain",
        "file_version": 1,
        "folder_id": "uuid2",
        "expected_size": 1024
    }
]


// API will return with
[
    {
        "base_url": "https://example.com",
        "token": "example-token",
        "file_name": "example.txt",
        "folder_id": "uuid",
        "file_version": 1,
        "ttl": 3600,
        "expires_at": "2024-06-01T12:00:00Z"
    },
    {
        "base_url": "https://example-2.com",
        "token": "example-token",
        "file_name": "example-2.txt",
        "folder_id": "uuid2",
        "file_version": 1,
        "ttl": 3600,
        "expires_at": "2024-06-01T12:00:00Z"
    }
]



// Example request payload for generating signed URLs

// TTL will be provided in the path parameter
[
    {
        "file_name": "example.txt",
        "file_type": "text/plain",
        "file_version": 1,
        "file_id": "uuid",  // If version is >2 the file_id will still be same
        "max_file_size": 1024 + 5%, // Adding 5% buffer to the expected size
        "file_path": "/data/org-id/user-id/first-3-characters-of-file-id/file-id/1"
    },
    {
        "file_name": "example2.txt",
        "file_type": "text/plain",
        "file_version": 3,
        "file_id": "uuid",  // If version is >2 the file_id will still be same
        "max_file_size": 1024 + 5%, // Adding 5% buffer to the expected size
        "file_path": "/data/org-id/user-id/first-3-characters-of-file-id/file-id/3"
    }
]


- Upload

- Download

- Delete

- Replace


---
UI - Alen

{
    "file_name": "example.txt",
    "file_type": "text/plain",
    "expected_size": 1024      // Bytes
}





---
Chetan API call

{
    file id
    max size
    folder path
    file name for verification
    TTL for the expiry
}


Callback's - Success, fail, timeout
