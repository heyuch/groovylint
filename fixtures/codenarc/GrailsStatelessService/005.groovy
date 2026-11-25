
          class MyService {
            DataSource dataSource
            OtherService otherService
            static scope = 'session'
            static transactional = false
            Sessionfactory sessionFactory
            GrailsApplication grailsApplication
          }
        