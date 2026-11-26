
                try (Sql sql = new Sql(null)) {
                    // do something
                } catch (Exception ex) {
                    // something else
                }
                
                try (Sql sql = new Sql(null)) {
                    println 123
                } catch (IOException e) {
                    log.error("error", e)
                } catch (Exception e) {
                    throw new RuntimeException(e)
                } finally {
                    pprintln 'done'
                }
            