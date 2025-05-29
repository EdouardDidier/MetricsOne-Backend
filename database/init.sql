/* Drop default database */
DROP DATABASE IF EXISTS postgres;


BEGIN;


DROP TABLE IF EXISTS public.drivers_images;
DROP TABLE IF EXISTS public.drivers;

CREATE TABLE IF NOT EXISTS public.drivers
(
    id serial,
    uuid UUID DEFAULT gen_random_uuid() UNIQUE NOT NULL,
    first_name character varying(127) NOT NULL,
    last_name character varying(127) NOT NULL,
    full_name character varying(255) GENERATED ALWAYS AS (INITCAP(first_name) || ' ' || UPPER(last_name)) STORED NOT NULL,
    url character varying(255) GENERATED ALWAYS AS (LOWER(first_name || '-' || last_name)) STORED NOT NULL,
    number integer NOT NULL,
    year integer NOT NULL,
    reference character varying(255) NOT NULL,
    team_id integer NOT NULL,
    PRIMARY KEY (id)
)
WITH (
    OIDS = FALSE
);



DROP TABLE IF EXISTS public.teams_images;
DROP TABLE IF EXISTS public.teams;

CREATE TABLE IF NOT EXISTS public.teams
(
    id serial PRIMARY KEY,
    uuid UUID DEFAULT gen_random_uuid() UNIQUE NOT NULL,
    name character varying(255) NOT NULL,
    url character varying(255) GENERATED ALWAYS AS (LOWER(REPLACE(name, ' ', '-'))) STORED NOT NULL,
    colour character(6) NOT NULL,
    year integer NOT NULL,
    reference character varying(255) NOT NULL
)
WITH (
    OIDS = FALSE
);

ALTER TABLE IF EXISTS public.drivers
    ADD FOREIGN KEY (team_id)
    REFERENCES public.teams (id) MATCH SIMPLE
    ON UPDATE NO ACTION
    ON DELETE NO ACTION
    NOT VALID;



DROP TABLE IF EXISTS public.sessions;
DROP TABLE IF EXISTS public.meetings;

CREATE TABLE IF NOT EXISTS public.meetings
(
    id serial PRIMARY KEY,
    uuid UUID DEFAULT gen_random_uuid() UNIQUE NOT NULL,
    key integer UNIQUE NOT NULL,
    number integer NOT NULL,
    location character varying(255) NOT NULL,
    official_name character varying(255) NOT NULL,
    name character varying(255) NOT NULL,
    url character varying(255) GENERATED ALWAYS AS (LOWER(REPLACE(name, ' ', '-'))) STORED NOT NULL,
    year integer NOT NULL
)
WITH (
    OIDS = FALSE
);




CREATE TABLE IF NOT EXISTS public.sessions
(
    id serial PRIMARY KEY,
    uuid UUID DEFAULT gen_random_uuid() UNIQUE NOT NULL,
    key integer UNIQUE NOT NULL,
    kind character varying(255) NOT NULL,
    name character varying(255) NOT NULL,
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ NOT NULL,
    path character varying(255) NOT NULL,
    meeting_key integer NOT NULL
)
WITH (
    OIDS = FALSE
);

ALTER TABLE IF EXISTS public.sessions
    ADD FOREIGN KEY (meeting_key)
    REFERENCES public.meetings (key) MATCH SIMPLE
    ON UPDATE NO ACTION
    ON DELETE NO ACTION
    NOT VALID;



CREATE TABLE IF NOT EXISTS public.teams_images
(
  team_id integer unique NOT NULL,
  car_url character varying(255) NOT NULL,
  logo_url character varying(255) NOT NULL
)
WITH (
    OIDS = FALSE
);

ALTER TABLE IF EXISTS public.teams_images
  ADD FOREIGN KEY (team_id)
  REFERENCES public.teams (id) MATCH SIMPLE
  ON UPDATE NO ACTION
  ON DELETE NO ACTION
  NOT VALID;



CREATE TABLE IF NOT EXISTS public.drivers_images
(
  driver_id integer unique NOT NULL,
  headshot_url character varying(255) NOT NULL,
  profile_url character varying(255) NOT NULL
)
WITH (
    OIDS = FALSE
);

ALTER TABLE IF EXISTS public.drivers_images
  ADD FOREIGN KEY (driver_id)
  REFERENCES public.drivers (id) MATCH SIMPLE
  ON UPDATE NO ACTION
  ON DELETE NO ACTION
  NOT VALID;



/* //// ////////////// //// */
/* //// Default values //// */
/* //// ////////////// //// */
INSERT INTO teams (name, colour, year, reference)
  VALUES
    /* 1  */ ('Alpine',           '0093CC', 2025, 'alpine'),           
    /* 2  */ ('Aston Martin',     '229971', 2025, 'aston martin'),
    /* 3  */ ('Ferrari',          'E80020', 2025, 'ferrari'),
    /* 4  */ ('Haas',             'B6BABD', 2025, 'haas'),
    /* 5  */ ('Kick Sauber',      '52E252', 2025, 'kick sauber'),
    /* 6  */ ('McLaren',          'FF8000', 2025, 'mclaren'),
    /* 7  */ ('Mercedes',         '27F4D2', 2025, 'mercedes'),       
    /* 8  */ ('Racing Bulls',     '6692FF', 2025, 'racing bulls'),
    /* 9  */ ('Red Bull Racing',  '3671C6', 2025, 'red bull'),
    /* 10 */ ('Williams',         '64C4FF', 2025, 'williams');

INSERT INTO drivers (first_name, last_name, number, year, reference, team_id)
  VALUES 
    ('Alexander', 'Albon',      23, 2025, 'alealb01', 10),
    ('Fernando',  'Alonso',     14, 2025, 'feralo01', 2),
    ('Kimi',      'Antonelli',  12, 2025, 'andant01', 7),
    ('Oliver',    'Bearman',    87, 2025, 'olibea01', 4),
    ('Gabriel',   'Bortoleto',  5,  2025, 'gabbor01', 5),
    ('Franco',    'Colapinto',  43, 2025, 'fracol01', 1),
  -- ('Jack',      'Doohan',     7,  2025, 'jacdoo01', 1), 
    ('Pierre',    'Gasly',      10, 2025, 'piegas01', 1),
    ('Isack',     'Hadjar',     6,  2025, 'isahad01', 8),
    ('Lewis',     'Hamilton',   44, 2025, 'lewham01', 3),
    ('Nico',      'Hulkenberg', 27, 2025, 'nichul01', 5),
    ('Liam',      'Lawson',     30, 2025, 'lialaw01', 8),
    ('Charles',   'Leclerc',    16, 2025, 'chalec01', 3),
    ('Lando',     'Norris',     4,  2025, 'lannor01', 6),
    ('Esteban',   'Ocon',       31, 2025, 'estoco01', 4),
    ('Oscar',     'Piastri',    81, 2025, 'oscpia01', 6),
    ('George',    'Russell',    63, 2025, 'georus01', 7),
    ('Carlos',    'Sainz',      55, 2025, 'carsai01', 10),
    ('Lance',     'Stroll',     18, 2025, 'lanstr01', 2),
    ('Yuki',      'Tsunoda',    22, 2025, 'yuktsu01', 9),
    ('Max',       'Verstappen', 1,  2025, 'maxver01', 9);

INSERT INTO teams_images (team_id, car_url, logo_url)
  VALUES
    (1,  '/d_team_car_fallback_image.png/content/dam/fom-website/teams/2025/alpine',          
      '/image/upload/content/dam/fom-website/2018-redesign-assets/team logos/alpine'),
    (2,  '/d_team_car_fallback_image.png/content/dam/fom-website/teams/2025/aston-martin',    
      '/image/upload/content/dam/fom-website/2018-redesign-assets/team logos/aston martin'),
    (3,  '/d_team_car_fallback_image.png/content/dam/fom-website/teams/2025/ferrari',         
      '/image/upload/content/dam/fom-website/2018-redesign-assets/team logos/ferrari'),
    (4,  '/d_team_car_fallback_image.png/content/dam/fom-website/teams/2025/haas',            
      '/image/upload/content/dam/fom-website/2018-redesign-assets/team logos/haas'),
    (5,  '/d_team_car_fallback_image.png/content/dam/fom-website/teams/2025/kick-sauber',     
      '/image/upload/content/dam/fom-website/2018-redesign-assets/team logos/kick sauber'),
    (6,  '/d_team_car_fallback_image.png/content/dam/fom-website/teams/2025/mclaren',         
      '/image/upload/content/dam/fom-website/2018-redesign-assets/team logos/mclaren'),
    (7,  '/d_team_car_fallback_image.png/content/dam/fom-website/teams/2025/mercedes',        
      '/image/upload/content/dam/fom-website/2018-redesign-assets/team logos/mercedes'),
    (8,  '/d_team_car_fallback_image.png/content/dam/fom-website/teams/2025/racing-bulls',    
      '/image/upload/fom-website/2018-redesign-assets/team%20logos/racing bulls'),
    (9,  '/d_team_car_fallback_image.png/content/dam/fom-website/teams/2025/red-bull-racing', 
      '/image/upload/content/dam/fom-website/2018-redesign-assets/team logos/red bull'),
    (10, '/d_team_car_fallback_image.png/content/dam/fom-website/teams/2025/williams',        
      '/image/upload/content/dam/fom-website/2018-redesign-assets/team logos/williams');

INSERT INTO drivers_images (driver_id, headshot_url, profile_url)
  VALUES
    (1,  '/d_driver_fallback_image.png/content/dam/fom-website/drivers/A/ALEALB01_Alexander_Albon/alealb01.avif',   
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Albon'),
    (2,  '/d_driver_fallback_image.png/content/dam/fom-website/drivers/F/FERALO01_Fernando_Alonso/feralo01.avif',   
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Alonso'),
    (3,  '/d_driver_fallback_image.png/content/dam/fom-website/drivers/K/ANDANT01_Kimi_Antonelli/andant01.avif',    
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Antonelli'),
    (4,  '/d_driver_fallback_image.png/content/dam/fom-website/drivers/O/OLIBEA01_Oliver_Bearman/olibea01.avif',    
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Bearman'),
    (5,  '/d_driver_fallback_image.png/content/dam/fom-website/drivers/G/GABBOR01_Gabriel_Bortoleto/gabbor01.avif', 
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Bortoleto'),
    (6,  '/d_driver_fallback_image.png/content/dam/fom-website/drivers/F/FRACOL01_Franco_Colapinto/fracol01.avif',  
      '/image/upload/fom-website/drivers/2025Drivers/colapinto'),
    (7,  '/d_driver_fallback_image.png/content/dam/fom-website/drivers/P/PIEGAS01_Pierre_Gasly/piegas01.avif',      
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Gasly'),
    (8,  '/d_driver_fallback_image.png/content/dam/fom-website/drivers/I/ISAHAD01_Isack_Hadjar/isahad01.avif',      
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Hadjar'),
    (9,  '/d_driver_fallback_image.png/content/dam/fom-website/drivers/L/LEWHAM01_Lewis_Hamilton/lewham01.avif',    
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Hamilton'),
    (10, '/d_driver_fallback_image.png/content/dam/fom-website/drivers/N/NICHUL01_Nico_Hulkenberg/nichul01.avif',   
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Hulkenberg'),
    (11, '/d_driver_fallback_image.png/content/dam/fom-website/drivers/L/LIALAW01_Liam_Lawson/lialaw01.avif',       
      '/image/upload/fom-website/drivers/2025Drivers/lawson-racing-bulls'),
    (12, '/d_driver_fallback_image.png/content/dam/fom-website/drivers/C/CHALEC01_Charles_Leclerc/chalec01.avif',   
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Leclerc'),
    (13, '/d_driver_fallback_image.png/content/dam/fom-website/drivers/L/LANNOR01_Lando_Norris/lannor01.avif',      
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Norris'),
    (14, '/d_driver_fallback_image.png/content/dam/fom-website/drivers/E/ESTOCO01_Esteban_Ocon/estoco01.avif',      
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Ocon'),
    (15, '/d_driver_fallback_image.png/content/dam/fom-website/drivers/O/OSCPIA01_Oscar_Piastri/oscpia01.avif',     
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Piastri'),
    (16, '/d_driver_fallback_image.png/content/dam/fom-website/drivers/G/GEORUS01_George_Russell/georus01.avif',    
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Russell'),
    (17, '/d_driver_fallback_image.png/content/dam/fom-website/drivers/C/CARSAI01_Carlos_Sainz/carsai01.avif',      
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Sainz'),
    (18, '/d_driver_fallback_image.png/content/dam/fom-website/drivers/L/LANSTR01_Lance_Stroll/lanstr01.avif',      
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Stroll'),
    (19, '/d_driver_fallback_image.png/content/dam/fom-website/drivers/Y/YUKTSU01_Yuki_Tsunoda/yuktsu01.avif',      
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Tsunoda'),
    (20, '/d_driver_fallback_image.png/content/dam/fom-website/drivers/M/MAXVER01_Max_Verstappen/maxver01.avif',    
      '/image/upload/content/dam/fom-website/drivers/2025Drivers/Verstappen');



END;
