### File Upload

Follow this: https://developers.cloudflare.com/r2/examples/rclone/.

Will need to generate an access token as well: https://developers.cloudflare.com/r2/data-access/s3-api/tokens/.

rclone config:

```
[sc2replays]
type = s3
provider = Cloudflare
access_key_id = <cf token id>
secret_access_key = <cf token private key>
region = auto
endpoint = https://<cf account id>.r2.cloudflarestorage.com
```

You can either copy this or follow the rclone setup like described in Cloudflare's docs.

Copy files with rclone: https://rclone.org/commands/rclone_copy/.

```
rclone copy <dir name> sc2replays:<bucket name> --progress --dry-run
```

Remove `--dry-run` when you are ready to upload.
