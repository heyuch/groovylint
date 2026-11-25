
            ((Node)o).children().inject([:]){ Map<String, String> m, Node childNode ->
                println m
            }
            (0..1).inject([:]){ Map<String, String> m, Node childNode ->
                println m
            }
        