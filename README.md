## Config
Json config example:
```json
{
  "config_format_version": "0.0.1",
  "working_directiries_path": "instances",
  "dedicated_server_dir": "../game/bin",
  "matchmaker_port": 12345
}
```
Fields:
- `config_format_version` - version of config format (used for future compatibility of your config)
- `working_directiries_path` - directory where the matchmaker will create working directories for each instance
- `dedicated_server_dir` - path to the dedicated server directory (assumed to be read-only)
- `matchmaker_port` - port that the matchmaker will listen to for incoming connections
