
            try {
                def a = 0
            } catch (ThreadDeath td) {
                try { println 4 } catch (ThreadDeath ttdd) { throw ttdd }
                throw td
            }
        