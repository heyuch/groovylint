
            try {
                def x = null
                return x
            } catch (IOException e) {
                return null
            } catch (Exception e) {
                LOG.error(e.getMessage())
                return null
            }
        