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

# Architecture

## Primitives

**Build**: list of buildings which form a build that can be represented as a `Vec<String>`, or a `String` with buildings comma-separated.

Ex:

```
"Gateway,Nexus,CyberneticsCore"

["Gateway", "Nexus", "CyberneticsCore"]
```

**Build Prefix**: a prefix that describes the race and matchup of the build in the form of `<race>-<matchup>`, where the matchup is the two races, lexographically sorted comma-separated.

Ex:

```
Protoss-Protoss,Zerg

Terran-Terran,Protoss
```

**Separators**: tokens which separate different pieces of information in identifiers

- Section separator: `"__"`
- Token separator: `':'`
- Building separator: `","`
- Build separator: `"--"`

## Clustering

Builds are clustered using [hierarchical agglomerative clustering](https://en.wikipedia.org/wiki/Hierarchical_clustering).

At the beginnging of the clustering process each build is it's own cluster. Clusters are merged if the distance between them is less than the defined maximum.

Distance is measured by computing the sequence difference of the cluster builds, then calculating the total information difference for the sequence difference. For each building in the sequence difference, an information value is calculated based on the independent probability of that building in the matchup.

One caveat to the information calculation is that from the 6th building onwards, there is a multipler applied to the information value to reduce the weight of the building. This reduction starts at 0.8 and linearly decreases by 0.2 each subsequent building position. The 10th building has a multiplier of 0.0.

The purpose of this reduction is to reduce the weight of buildings later on in the build, since there are naturally branching paths and we would like builds with slight deviations to be clustered together.

- Nested iteration through all builds
  - Skip builds that are the same since they have difference of 0.0
  - Check prefix of all builds to ensure only comparing builds of same race and matchup
  - Generate stable comparison identifier from both builds by lexographically sorting the builds
  - If we already have a record for this build, continue (Once for (inner, outer) and (outer, inner))
  - Find sequence matches between builds, then use to calculate missing buildings
  - Calculate tf-idf for missing buildings and save total information difference as a build comparison
- 
