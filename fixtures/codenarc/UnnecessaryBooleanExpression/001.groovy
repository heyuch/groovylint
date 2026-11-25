
            def ready = value && Boolean.TRUE
            if (value || Boolean.TRUE) {
                println 'ok'
            }
            def result = value && Boolean.FALSE

            ready = Boolean.TRUE && value
            if (Boolean.TRUE || value) { println 'ok' }
            result = Boolean.FALSE && value
        