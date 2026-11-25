
            (x == y) ? same : diff
            (x) ? same : diff

            // because of GroovyTruth, there is no inverse of != null
            (x != null) ? diff : same
            (null != x) ? diff : same

            // because of GroovyTruth, there is no inverse of != true
            (x != true) ? diff : same
            (true != x) ? diff : same

            // because of GroovyTruth, there is no inverse of != true
            (x != false) ? diff : same
            (false != x) ? diff : same
        