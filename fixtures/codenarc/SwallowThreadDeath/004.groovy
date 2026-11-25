
            def other = new ThreadDeath()
            try {
                def a = 0
            } catch (ThreadDeath td) {
                throw other
            }
        