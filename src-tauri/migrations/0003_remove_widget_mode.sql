UPDATE settings
SET value = json_remove(value, '$.widget_mode')
WHERE key = 'preferences';
