
            System.runFinalizersOnExit(true)
            def method = {
                System.runFinalizersOnExit(false)
            }
        