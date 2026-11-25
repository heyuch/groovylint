
            try {
                doStuff()
            } catch(DaoException e) {
                log.warning("Ooops", e)
                new ServiceException(e)
            } catch(Exception e) {
                new SystemException(e)
            }
        