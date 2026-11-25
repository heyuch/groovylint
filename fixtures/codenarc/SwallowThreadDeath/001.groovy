
            try {
                def a = 0
            } catch (ThreadDeath td) {
                throw new ThreadDeath()
            }
        