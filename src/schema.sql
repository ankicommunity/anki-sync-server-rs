CREATE TABLE IF NOT EXISTS media (
                       fname TEXT NOT NULL PRIMARY KEY,
                       usn INT NOT NULL,
                       csum TEXT -- null if deleted
                   );
              
