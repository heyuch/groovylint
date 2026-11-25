
            import javax.servlet.http.*
            import javax.sql.*
            import com.example.OtherClass

            class MyClass {
                void doStuff(javax.servlet.http.Cookie cookie, Cookie[] cookies) {
                    def dataSource = new javax.sql.DataSource()
                    DataSource dataSource2 = wrap(dataSource)
                }
            }
        