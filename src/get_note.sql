SELECT id,
  guid,
  mid,
  mod,
  ?,
  tags,
  flds,
  cast(sfld AS text),
  csum
FROM notes 
WHERE usn = -1