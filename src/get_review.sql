SELECT id,
  cid,
  ?,
  ease,
  cast(ivl AS integer),
  cast(lastIvl AS integer),
  factor,
  time,
  type
FROM revlog 
where usn = -1