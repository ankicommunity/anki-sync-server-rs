SELECT queue,
  id,
  nid,
  due,
  cast(ivl AS integer),
  cast(mod AS integer),
  did,
  odid
FROM cards
WHERE did IN (
    SELECT id
    FROM active_decks
  )
  AND (
    queue IN (2, 3)
    AND due <= ?
  )