
                try (Sql sql = new Sql(null)) {
                    // do something
                }
                
                try (Sql sql = new Sql(null)) {
                    println 123
                } finally {
                    println 'done'
                }
                
                try (def input = url.openStream()
                     def output = new ByteArrayOutputStream(4096)) {
                     // some code here to work with streams
                }
            